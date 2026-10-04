//! Étape Unit — tests unitaires criterion (Task 8).
//!
//! Lance `make tests_run` dans la salle blanche (LC_ALL=C, timeout
//! 120 s), relaie stdout/stderr en [`Event::LogLine`] et parse la
//! sortie criterion (2.x) — le vrai criterion écrit TOUT sur stderr ;
//! les deux flux sont parsés (une règle exotique peut rediriger) :
//! - lignes de succès : contiennent `[OK]` (fixture scriptée) ou
//!   `[PASS]` (vrai criterion `--verbose`) — les codes ANSI
//!   (`\x1b[...m`) sont strippés avant parsing ;
//! - lignes d'échec : contiennent `[KO]` ou `[FAIL]` ;
//! - crash : « [FAIL] name: CRASH! » — le suffixe « CRASH! » est
//!   strippé du nom et le verdict est `TestVerdict::Crashed(-1)`
//!   (criterion ne donne pas le signal sur cette ligne : -1 =
//!   signal inconnu à ce niveau de parsing) ;
//! - résumé final : « Tests: X | Passing: Y | Failing: Z » (tolérant
//!   aux variations d'espaces/casse minimales — le « Synthesis:
//!   Tested: … » du vrai criterion est reconnu aussi).
//!
//! Chaque test parsé émet `TestStarted`/`TestFinished` (groupe
//! [`Step::Unit`]) et alimente `ctx.tests` pour le rapport. Criterion
//! ne fournit pas de diff exploitable : un `Failed` porte des champs
//! vides — le détail vit dans les LogLine relayées.
//!
//! Verdict : l'étape réussit ssi exit==0 ET aucun test en échec ET au
//! moins un test attesté (parsé, ou résumé `Tested > 0`). Le vrai
//! criterion SANS `--verbose` n'émet QUE la Synthesis pour une suite
//! 100 % verte (aucun `[PASS]`) : le résumé conduit alors le verdict
//! (check récapitulatif « N tests, M en échec (résumé) ») au lieu
//! d'un faux KO « sans test détecté ». Et comme il n'émet que les
//! `[FAIL]` dès qu'un test échoue, un enregistrement de vérité du
//! résumé (« résumé criterion : P pass / F fail ») est émis EN PLUS
//! des tests parsés — sinon le décompte global mentirait (« 0/1 »
//! pour une suite 3 pass / 1 fail). Un `make tests_run` en échec ne
//! masque PAS les tests déjà parsés (la vraie moulinette montre les
//! tests passés même si la suite échoue) ; un tests_run sans test
//! attesté est un KO explicite (un tests_run vide est un zero
//! déguisé) ; une sortie tronquée (stdout ou stderr) est un KO
//! explicite : le parsing EST le verdict quand la règle sort 0, et
//! des lignes perdues peuvent cacher des échecs.
//!
//! Couverture : si `gcovr` est présent (sonde `--version` bornée à
//! 5 s) ET des `.gcda` existent dans la salle blanche,
//! `gcovr --print-summary` (timeout 30 s) est relayé en LogLine.
//! Jamais bloquant : toute erreur gcovr est relayée « gcovr: … » et
//! ignorée.
//!
//! Cas particuliers :
//! - `tests_run_rule == false` → l'étape est désactivée :
//!   `StepFinished ok:true, skipped:true` ([`super::step_disabled`]) ;
//! - build absent (ctx construit à la main — en pipeline, `run_step`
//!   garantit un build OK avant Unit) → KO explicite, jamais un vert
//!   à vide.

use super::events::{Event, Step, TestVerdict};
use super::{tail, Collect, PipelineContext, STDERR_TAIL};
use crate::exec::{run_capture, ExecStatus, Limits};
use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::mpsc;
use std::sync::LazyLock;
use std::time::Duration;

/// Timeout du `make tests_run` (spec : 120 s, comme le build).
const TESTS_RUN_TIMEOUT: Duration = Duration::from_secs(120);

/// Timeout de la sonde `gcovr --version`.
const GCOVR_PROBE_TIMEOUT: Duration = Duration::from_secs(5);

/// Timeout de `gcovr --print-summary`.
const GCOVR_TIMEOUT: Duration = Duration::from_secs(30);

/// Profondeur maximale de la recherche de `.gcda` — même borne que la
/// copie en salle blanche (build.rs).
const MAX_GCDA_DEPTH: usize = 64;

/// Étape 5 du pipeline : tests unitaires. Renvoie `true` ssi exit==0,
/// aucun test en échec, au moins un test attesté (parsé, ou résumé
/// `Tested > 0`) et sortie non tronquée (désactivation et skip de
/// dépendance exceptés).
pub fn run(ctx: &mut PipelineContext, tx: &mpsc::Sender<Event>) -> bool {
    super::step_started(tx, Step::Unit);

    // Règle absente de la batterie : le check n'a pas vocation à
    // tourner (skip de config — ni échec, ni skip de dépendance).
    if !ctx.battery.project.tests_run_rule {
        super::step_disabled(
            ctx,
            tx,
            Step::Unit,
            "skipped: pas de règle tests_run".to_string(),
        );
        return true;
    }

    let mut c = Collect::new(tx, Step::Unit);

    // Sans salle blanche, rien à lancer : KO explicite (en pipeline,
    // run_step a déjà skip Unit si le build a échoué — ce cas ne vient
    // que d'un ctx construit à la main).
    let Some(build) = &ctx.build else {
        c.check(
            "tests_run",
            false,
            "pas de salle blanche (build absent)".to_string(),
        );
        let (_, _, checks) = c.finish();
        super::step_finished(
            ctx,
            tx,
            Step::Unit,
            false,
            "build absent".to_string(),
            checks,
        );
        return false;
    };
    // Chemin possédé : ctx est re-emprunté en mutable plus bas
    // (ctx.tests) pendant que la salle blanche y vit.
    let white = build.dir.path().to_path_buf();

    let Some(outcome) = run_tests_run(&white, &mut c) else {
        let (_, _, checks) = c.finish();
        super::step_finished(
            ctx,
            tx,
            Step::Unit,
            false,
            "make tests_run : lancement impossible".to_string(),
            checks,
        );
        return false;
    };
    c.log(&outcome.stdout);
    c.log(&outcome.stderr);

    // Parse stdout puis stderr (le vrai criterion écrit TOUT sur
    // stderr ; une règle exotique peut rediriger — on ne perd jamais
    // un test).
    let mut parsed = parse_output(&outcome.stdout);
    let err_parsed = parse_output(&outcome.stderr);
    parsed.tests.extend(err_parsed.tests);
    parsed.summary = parsed.summary.or(err_parsed.summary);

    // Chaque test parsé : Started/Finished + record pour le rapport.
    let mut failed = 0usize;
    for (name, parsed_verdict) in &parsed.tests {
        let verdict = match parsed_verdict {
            ParsedVerdict::Passed => TestVerdict::Passed,
            ParsedVerdict::Failed => {
                failed += 1;
                // Criterion ne donne pas de diff exploitable : le
                // détail vit dans les LogLine relayées.
                TestVerdict::Failed {
                    diff: String::new(),
                    expected: String::new(),
                    got: String::new(),
                }
            }
            // « CRASH! » ne précise pas le signal : -1 = signal
            // inconnu à ce niveau de parsing.
            ParsedVerdict::Crashed => {
                failed += 1;
                TestVerdict::Crashed(-1)
            }
        };
        super::send(
            tx,
            Event::TestStarted {
                group: Step::Unit.name().to_string(),
                name: name.clone(),
            },
        );
        super::send(
            tx,
            Event::TestFinished {
                group: Step::Unit.name().to_string(),
                name: name.clone(),
                result: verdict.clone(),
            },
        );
        ctx.tests.push(super::verdict::test_record(
            Step::Unit.name(),
            name,
            &verdict,
        ));
    }

    // Verdict : exit de la règle, puis parsing.
    let mut ok = match outcome.status {
        ExecStatus::Exit(0) => c.check("tests_run", true, "make tests_run ok".to_string()),
        status => {
            let stderr_tail = tail(&outcome.stderr, STDERR_TAIL);
            let detail = if stderr_tail.is_empty() {
                format!("make tests_run ({})", super::build::describe_status(status))
            } else {
                format!(
                    "make tests_run ({})\n{stderr_tail}",
                    super::build::describe_status(status)
                )
            };
            c.check("tests_run", false, detail)
        }
    };
    // Sortie tronquée : le parsing EST le verdict quand la règle sort
    // 0 — des lignes perdues peuvent cacher des échecs. KO explicite.
    if outcome.stdout_truncated || outcome.stderr_truncated {
        ok &= c.check(
            "sortie",
            false,
            "sortie tronquée : parsing potentiellement incomplet".to_string(),
        );
    }
    ok &= match (parsed.tests.is_empty(), parsed.summary) {
        // Non-verbose : le vrai criterion n'émet QUE la Synthesis pour
        // une suite 100 % verte (aucun [PASS]). Le résumé atteste
        // Tested > 0 : il conduit le verdict.
        (true, Some((tested, _, failing))) if tested > 0 => c.check(
            "tests",
            failing == 0,
            format!("{tested} tests, {failing} en échec (résumé)"),
        ),
        // Ni test parsé ni résumé attestant des tests : un tests_run
        // vide est un zero déguisé.
        (true, _) => c.check("tests", false, "tests_run sans test détecté".to_string()),
        (false, summary) => {
            let passed = parsed.tests.len() - failed;
            let mut all = c.check(
                "tests",
                failed == 0,
                format!("{passed} passed, {failed} failed"),
            );
            if let Some((tested, passing, failing)) = summary {
                // Vérité du résumé, EN PLUS des tests parsés : en
                // non-verbose criterion n'émet que les [FAIL] — sans
                // ce check, le décompte global mentirait (« 0/1 »
                // pour une suite 3 pass / 1 fail).
                all &= c.check(
                    "résumé",
                    failing == 0,
                    format!(
                        "résumé criterion : {passing} pass / {failing} fail (tested: {tested})"
                    ),
                );
            }
            all
        }
    };

    // Couverture gcovr : jamais bloquante, n'influe pas le verdict.
    coverage(&white, &c);

    let summary = match (parsed.tests.is_empty(), parsed.summary) {
        (true, Some((tested, _, failing))) if tested > 0 => {
            format!("{tested} tests, {failing} en échec (résumé)")
        }
        (true, _) => "tests_run sans test détecté".to_string(),
        (false, _) => format!("{} tests, {failed} en échec", parsed.tests.len()),
    };
    let (_, _, checks) = c.finish();
    super::step_finished(ctx, tx, Step::Unit, ok, summary, checks);
    ok
}

/// Lance `env LC_ALL=C make tests_run` dans `dir` (borné à
/// [`TESTS_RUN_TIMEOUT`]). Un échec de spawn est un check KO et
/// renvoie `None` — tout le reste (exit≠0, signal, timeout) laisse
/// l'outcome à l'appelant : les tests déjà émis restent parsables.
fn run_tests_run(dir: &Path, c: &mut Collect) -> Option<crate::exec::ExecOutcome> {
    let args = vec![
        "LC_ALL=C".to_string(),
        "make".to_string(),
        "tests_run".to_string(),
    ];
    match run_capture(
        Path::new("/usr/bin/env"),
        &args,
        "",
        dir,
        TESTS_RUN_TIMEOUT,
        Limits::default(),
    ) {
        Ok(outcome) => Some(outcome),
        Err(e) => {
            c.check(
                "tests_run",
                false,
                format!("make tests_run (lancement impossible : {e:#})"),
            );
            None
        }
    }
}

/// Couverture gcovr : si gcovr répond à `--version` ET un `.gcda`
/// existe sous `dir`, relaie `gcovr --print-summary` en LogLine.
/// Jamais bloquant : toute erreur est relayée « gcovr: … » et ignorée.
fn coverage(dir: &Path, c: &Collect) {
    if !gcovr_present(dir) || !has_gcda(dir) {
        return;
    }
    let args = vec!["--print-summary".to_string()];
    match run_capture(
        Path::new("gcovr"),
        &args,
        "",
        dir,
        GCOVR_TIMEOUT,
        Limits::default(),
    ) {
        Ok(outcome) => {
            c.log(&outcome.stdout);
            c.log(&outcome.stderr);
        }
        Err(e) => c.log(&format!("gcovr: {e:#}")),
    }
}

/// `true` si gcovr répond à `--version` (sonde bornée à
/// [`GCOVR_PROBE_TIMEOUT`]) — absent, muet ou tué : non présent.
fn gcovr_present(dir: &Path) -> bool {
    run_capture(
        Path::new("gcovr"),
        &["--version".to_string()],
        "",
        dir,
        GCOVR_PROBE_TIMEOUT,
        Limits::default(),
    )
    .is_ok_and(|o| matches!(o.status, ExecStatus::Exit(0)))
}

/// `true` si un `.gcda` existe sous `dir` (marche itérative bornée —
/// la salle blanche peut contenir un arbre hostile).
fn has_gcda(dir: &Path) -> bool {
    let mut stack = vec![(dir.to_path_buf(), 0usize)];
    while let Some((d, depth)) = stack.pop() {
        let Ok(entries) = fs::read_dir(&d) else {
            continue; // dossier illisible : ignoré, jamais bloquant
        };
        for entry in entries.flatten() {
            let Ok(ftype) = entry.file_type() else {
                continue;
            };
            if ftype.is_dir() && depth < MAX_GCDA_DEPTH {
                stack.push((entry.path(), depth + 1));
            } else if ftype.is_file() && entry.file_name().to_string_lossy().ends_with(".gcda") {
                return true;
            }
        }
    }
    false
}

/// Sortie criterion parsée : les tests `(nom, verdict)` dans l'ordre
/// et le résumé final éventuel `(tests, passing, failing)`.
struct ParsedOutput {
    tests: Vec<(String, ParsedVerdict)>,
    summary: Option<(u64, u64, u64)>,
}

/// Verdict parsé d'une ligne de test criterion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParsedVerdict {
    Passed,
    Failed,
    /// « CRASH! » : criterion ne donne pas le signal sur la ligne.
    Crashed,
}

/// Codes ANSI (`\x1b[...m`) strippés avant parsing. Compilée une fois
/// pour toutes (LazyLock) — regex littérale, toujours valide.
static ANSI_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\x1b\[[0-9;]*m").expect("regex ANSI littérale valide"));

/// Résumé criterion « Test(s|ed): X | Passing: Y | Failing: Z »
/// (tolérant espaces/casse ; la forme « Synthesis: Tested: … » du
/// vrai criterion est reconnue). Compilée une fois (LazyLock).
static SUMMARY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)test(?:s|ed)?\s*:\s*(\d+)\s*\|\s*passing\s*:\s*(\d+)\s*\|\s*failing\s*:\s*(\d+)",
    )
    .expect("regex résumé littérale valide")
});

/// Parse une sortie criterion : codes ANSI strippés, marqueurs
/// `[OK]`/`[PASS]` (succès) et `[KO]`/`[FAIL]` (échec — ou crash si
/// le nom se termine par « CRASH! »), résumé « Tests: X | Passing: Y
/// | Failing: Z » (tolérant aux variations d'espaces/casse — le
/// « Tested: … » du vrai criterion est reconnu).
fn parse_output(output: &str) -> ParsedOutput {
    let mut parsed = ParsedOutput {
        tests: Vec::new(),
        summary: None,
    };
    for line in output.lines() {
        let line = ANSI_RE.replace_all(line, "");
        if let Some((name, verdict)) = parse_test_line(&line) {
            parsed.tests.push((name, verdict));
        } else if let Some(cap) = SUMMARY_RE.captures(&line) {
            // Chiffres capturés par \d+ : le parse ne peut échouer que
            // sur un entier absurde (> u64) — repli défensif à 0.
            parsed.summary = Some((
                cap[1].parse().unwrap_or(0),
                cap[2].parse().unwrap_or(0),
                cap[3].parse().unwrap_or(0),
            ));
        }
    }
    parsed
}

/// Marqueurs criterion : succès (`[OK]` fixture, `[PASS]` vrai
/// criterion --verbose) et échecs (`[KO]`, `[FAIL]`).
const MARKERS: &[(&str, bool)] = &[
    ("[OK]", true),
    ("[PASS]", true),
    ("[KO]", false),
    ("[FAIL]", false),
];

/// Si `line` rapporte un test, renvoie `(nom, verdict)`.
fn parse_test_line(line: &str) -> Option<(String, ParsedVerdict)> {
    let &(marker, passed) = MARKERS.iter().find(|(m, _)| line.contains(m))?;
    let (name, crashed) = extract_name(line, marker)?;
    let verdict = match (passed, crashed) {
        (true, _) => ParsedVerdict::Passed,
        (false, true) => ParsedVerdict::Crashed,
        (false, false) => ParsedVerdict::Failed,
    };
    Some((name, verdict))
}

/// Nom du test autour du marqueur, et `true` si criterion rapporte un
/// crash : « tests::strlen: [OK] » → (« tests::strlen », false) ;
/// « [PASS] sample::passing: (0.00s) » → (« sample::passing », false) ;
/// « [FAIL] sample::segv: CRASH! » → (« sample::segv », true).
fn extract_name(line: &str, marker: &str) -> Option<(String, bool)> {
    let without = line.replacen(marker, " ", 1);
    let mut name = without.trim();
    // Timing final du vrai criterion : « name: (0.00s) ».
    if name.ends_with(')') {
        if let Some(pos) = name.rfind('(') {
            name = name[..pos].trim_end();
        }
    }
    // Crash du vrai criterion : « name: CRASH! » (strippé après le
    // timing — une ligne de crash n'en porte pas).
    let crashed = name.ends_with("CRASH!");
    if crashed {
        name = name[..name.len() - "CRASH!".len()].trim_end();
    }
    let name = name.trim_end_matches(':').trim();
    if name.is_empty() {
        None
    } else {
        Some((name.to_string(), crashed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_marqueurs_ok_ko_fail() {
        let p = parse_output("tests::a: [OK]\ntests::b: [KO]\ntests::c: [FAIL]\n");
        assert_eq!(
            p.tests,
            vec![
                ("tests::a".to_string(), ParsedVerdict::Passed),
                ("tests::b".to_string(), ParsedVerdict::Failed),
                ("tests::c".to_string(), ParsedVerdict::Failed),
            ]
        );
    }

    #[test]
    fn parse_marqueur_pass_et_timing_du_vrai_criterion() {
        let p = parse_output("[PASS] sample::ok: (0.00s)\n[FAIL] sample::ko: (0.01s)\n");
        assert_eq!(
            p.tests,
            vec![
                ("sample::ok".to_string(), ParsedVerdict::Passed),
                ("sample::ko".to_string(), ParsedVerdict::Failed),
            ]
        );
    }

    #[test]
    fn parse_crash_suffixe_strippe_et_verdict_crashed() {
        // Format réel vérifié (criterion 2.4.3) : « [FAIL]
        // sample::segv: CRASH! » — le suffixe « CRASH! » est strippé
        // du nom, le verdict est Crashed.
        let p = parse_output("[FAIL] sample::segv: CRASH!\n");
        assert_eq!(
            p.tests,
            vec![("sample::segv".to_string(), ParsedVerdict::Crashed)]
        );
    }

    #[test]
    fn parse_strippe_les_codes_ansi() {
        let p = parse_output(
            "\x1b[32mtests::green: [OK]\x1b[0m\ntests::marker: \x1b[1;32m[OK]\x1b[0m\n\x1b[31mtests::red: [KO]\x1b[0m\n",
        );
        assert_eq!(
            p.tests,
            vec![
                ("tests::green".to_string(), ParsedVerdict::Passed),
                ("tests::marker".to_string(), ParsedVerdict::Passed),
                ("tests::red".to_string(), ParsedVerdict::Failed),
            ]
        );
    }

    #[test]
    fn parse_resume_tolerant_espaces_et_casse() {
        // Forme canonique de la consigne.
        let p = parse_output("Tests: 3 | Passing: 3 | Failing: 0\n");
        assert_eq!(p.summary, Some((3, 3, 0)));
        // Espaces et casse minimales.
        let p = parse_output("tests : 2|passing :1 | FAILING: 1\n");
        assert_eq!(p.summary, Some((2, 1, 1)));
        // Forme du vrai criterion 2.x.
        let p =
            parse_output("[====] Synthesis: Tested: 3 | Passing: 2 | Failing: 1 | Crashing: 0 \n");
        assert_eq!(p.summary, Some((3, 2, 1)));
    }

    #[test]
    fn parse_ignore_le_bruit() {
        // Les lignes criterion de diagnostic ([----], [====], [RUN ])
        // ne sont pas des tests.
        let p = parse_output(
            "[====] Criterion v2.4.3\n[RUN ] sample::a\n[----] t.c:5: Assertion Failed\n",
        );
        assert!(p.tests.is_empty());
        assert_eq!(p.summary, None);
    }
}
