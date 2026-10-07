//! Tests de l'étape Symbols (Task 7) : fonctions interdites détectées
//! dans les artefacts du build (`.o` Functions, binaire linké Binary).
//!
//! Les fixtures C sont compilées à la volée avec `cc` ; si `cc` est
//! absent du PATH, les tests qui compilent skippent (même convention
//! que le test banana de tests/prelim.rs). `-fno-builtin` partout :
//! gcc ne doit pas réécrire `printf("…\n")` en `puts` — les symboles
//! observés doivent être exactement ceux du source.
//!
//! L'étape est pilotée isolément : `ctx.build` est construit à la
//! main (salle blanche + artefacts), sans passer par l'étape Build.

mod common;

use all_seeing_eye::battery::Battery;
use all_seeing_eye::engine::events::{Event, Step};
use all_seeing_eye::engine::{symbols, BuildArtifacts, PipelineContext};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use tempfile::TempDir;

/// Batterie Functions sur les deliveries `<d>.c` demandées, whitelist
/// paramétrée (`allowed` = littéraux TOML, ex. `"write", "printf"`).
fn functions_toml(deliveries: &[&str], allowed: &str) -> String {
    let tasks: String = deliveries
        .iter()
        .map(|d| {
            format!(
                r#"
[[task]]
name = "{d}"
delivery = "{d}.c"
harness = "harness/{d}_main.c"
stdout = ""
"#
            )
        })
        .collect();
    format!(
        r#"[project]
name = "sym_fn"
type = "functions"
allowed_functions = [{allowed}]
{tasks}"#
    )
}

/// Batterie Binary (binaire `prog`), whitelist paramétrée.
fn binary_toml(allowed: &str) -> String {
    format!(
        r#"[project]
name = "sym_bin"
type = "binary"
binary = "prog"
allowed_functions = [{allowed}]
"#
    )
}

/// Écrit `toml` dans un tempdir et charge la batterie.
/// Le TempDir est renvoyé pour garder `battery.root` valide.
fn load_battery(toml: &str) -> (TempDir, Battery) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("moulinette.toml");
    fs::write(&path, toml).unwrap();
    (dir, Battery::load(&path).unwrap())
}

/// `true` si `cc` est disponible ; sinon les tests qui compilent
/// skippent (convention banana de tests/prelim.rs).
fn cc_present() -> bool {
    Command::new("cc").arg("--version").output().is_ok()
}

/// Écrit `<dir>/<name>.c` et le compile en `<dir>/<name>.o` — le
/// contrat build.rs (Task 5-6) que symbols consomme : un `.o` par
/// delivery, à la racine de la salle blanche, nommé d'après le stem.
fn compile_obj(dir: &Path, name: &str, src: &str) {
    fs::write(dir.join(format!("{name}.c")), src).unwrap();
    let status = Command::new("cc")
        .args([
            "-fno-builtin",
            "-c",
            &format!("{name}.c"),
            "-o",
            &format!("{name}.o"),
        ])
        .current_dir(dir)
        .status()
        .expect("lancement de cc impossible");
    assert!(status.success(), "cc -c {name}.c a échoué");
}

/// Compile et lie `<dir>/prog` depuis `main.c` ; renvoie son chemin.
fn compile_prog(dir: &Path, src: &str) -> PathBuf {
    fs::write(dir.join("main.c"), src).unwrap();
    let status = Command::new("cc")
        .args(["-fno-builtin", "main.c", "-o", "prog"])
        .current_dir(dir)
        .status()
        .expect("lancement de cc impossible");
    assert!(status.success(), "cc main.c -o prog a échoué");
    dir.join("prog")
}

/// Pose les artefacts donnés dans `ctx.build` et lance l'étape ;
/// renvoie le verdict, les événements et le ctx (qui garde la salle
/// blanche vivante le temps des assertions). Le `target` factice est
/// droppé : symbols ne lit jamais `ctx.target` (elle travaille sur
/// `ctx.build`).
fn run_symbols(
    battery: &Battery,
    build: Option<(TempDir, Option<PathBuf>)>,
) -> (bool, Vec<Event>, PipelineContext) {
    let target = TempDir::new().unwrap();
    let mut ctx = common::test_ctx(battery, target.path());
    ctx.build = build.map(|(dir, binary)| BuildArtifacts { dir, binary });
    let (tx, rx) = mpsc::channel();
    let ok = symbols::run(&mut ctx, &tx);
    drop(tx);
    (ok, rx.iter().collect(), ctx)
}

/// Extrait les checks `(name, ok, detail)` du flux ; tout check doit
/// être estampillé Symbols.
fn extract_checks(events: &[Event]) -> Vec<(String, bool, String)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::CheckFinished {
                step,
                name,
                ok,
                detail,
            } => {
                assert_eq!(*step, Step::Symbols, "check émis pour la mauvaise étape");
                Some((name.clone(), *ok, detail.clone()))
            }
            _ => None,
        })
        .collect()
}

/// Le `(ok, skipped, summary)` du StepFinished Symbols — exactement
/// un par run (l'invariant de l'orchestrateur vaut aussi à l'isolement).
fn step_finished(events: &[Event]) -> (bool, bool, String) {
    let mut found = events.iter().filter_map(|e| match e {
        Event::StepFinished {
            step: Step::Symbols,
            ok,
            skipped,
            summary,
        } => Some((*ok, *skipped, summary.clone())),
        _ => None,
    });
    let first = found.next().expect("StepFinished Symbols émis");
    assert!(found.next().is_none(), "plusieurs StepFinished Symbols");
    first
}

/// Appelle printf ET write (valeur de retour de write utilisée :
/// glibc le déclare __wur).
const PRINTF_WRITE_C: &str = r#"
#include <stdio.h>
#include <unistd.h>

void ex01(void)
{
    printf("salut");
    if (write(1, "x", 1) == -1)
        return;
}
"#;

/// Appelle printf, côté A.
const PRINTF_A_C: &str = "#include <stdio.h>\n\nvoid part_a(void)\n{\n    printf(\"a\");\n}\n";

/// Appelle printf, côté B.
const PRINTF_B_C: &str = "#include <stdio.h>\n\nvoid part_b(void)\n{\n    printf(\"b\");\n}\n";

/// Définit `helper` — la moitié « définition » de l'appel interne.
const HELPER_C: &str = "int helper(void)\n{\n    return 42;\n}\n";

/// Appelle `helper` — indéfini dans ce .o, défini dans l'autre.
const USE_IT_C: &str = "int helper(void);\n\nint use_it(void)\n{\n    return helper();\n}\n";

/// Hello world : référence printf + tout le runtime C (_start,
/// __libc_start_main…) une fois lié.
const HELLO_C: &str =
    "#include <stdio.h>\n\nint main(void)\n{\n    printf(\"hello world\\n\");\n    return 0;\n}\n";

/// fprintf sur stderr ET stdout : dans un .o, les streams libc sont
/// des indéfinis NOTYPE — des VARIABLES globales, pas des fonctions.
const FPRINTF_STREAMS_C: &str = r#"
#include <stdio.h>

void ex01(void)
{
    fprintf(stderr, "e");
    fprintf(stdout, "o");
}
"#;

/// Adresse d'une fonction weak : gcc charge l'adresse via la GOT,
/// d'où un indéfini `_GLOBAL_OFFSET_TABLE_` — bookkeeping compilateur,
/// pas une fonction appelable (émis même sans -fPIC sur x86-64).
const WEAK_GOT_C: &str = r#"
__attribute__((weak)) extern void weak_fn(void);

void ex01(void)
{
    if (__builtin_expect(weak_fn != 0, 1))
        weak_fn();
}
"#;

#[test]
fn whitelist_vide_desactive_l_etape() {
    // Pas besoin de cc : l'étape est désactivée avant tout artefact.
    let (_bat_dir, battery) = load_battery(&functions_toml(&["ex01"], ""));
    let (ok, events, ctx) = run_symbols(&battery, None);

    assert!(ok, "whitelist vide : l'étape désactivée réussit");
    assert!(matches!(
        events.first(),
        Some(Event::StepStarted {
            step: Step::Symbols,
            ..
        })
    ));
    let (ok_ev, skipped, summary) = step_finished(&events);
    assert!(ok_ev, "skip de config : ok:true (pas un échec)");
    assert!(skipped, "skip de config : skipped:true");
    assert!(
        summary.contains("skipped"),
        "résumé attendu « skipped… » : {summary:?}"
    );
    assert!(
        extract_checks(&events).is_empty(),
        "aucun check quand l'étape est désactivée : {events:?}"
    );
    // Le StepReport suit la même sémantique.
    let report = ctx.steps.last().expect("StepReport posé");
    assert_eq!(report.step, "symbols");
    assert!(report.ok && report.skipped, "StepReport : {report:?}");
}

#[test]
fn sans_artefacts_ok_rien_a_scanner() {
    // (a) build absent (ctx.build None).
    let (_bat_dir, battery_fn) = load_battery(&functions_toml(&["ex01"], "\"write\""));
    let (ok, events, _ctx) = run_symbols(&battery_fn, None);
    assert!(ok, "rien à scanner : succès à vide, pas un échec");
    let (ok_ev, skipped, summary) = step_finished(&events);
    assert!(
        ok_ev && !skipped && summary.contains("aucun artefact"),
        "attendu ok:true skipped:false « aucun artefact » : {summary:?}"
    );

    // (b) Binary sans binaire (build KO en amont).
    let (_bat_dir2, battery_bin) = load_battery(&binary_toml("\"write\""));
    let white = TempDir::new().unwrap();
    let (ok, events, _ctx) = run_symbols(&battery_bin, Some((white, None)));
    assert!(ok);
    let (ok_ev, skipped, summary) = step_finished(&events);
    assert!(
        ok_ev && !skipped && summary.contains("aucun artefact"),
        "Binary sans binaire : {summary:?}"
    );

    // (c) Functions sans aucun .o (deliveries non compilées).
    let white = TempDir::new().unwrap();
    let (ok, events, _ctx) = run_symbols(&battery_fn, Some((white, None)));
    assert!(ok);
    let (ok_ev, skipped, summary) = step_finished(&events);
    assert!(
        ok_ev && !skipped && summary.contains("aucun artefact"),
        "Functions sans .o : {summary:?}"
    );
}

#[test]
fn objet_printf_interdit_write_autorise_ko() {
    if !cc_present() {
        eprintln!("cc absent du PATH : test skippé");
        return;
    }
    let (_bat_dir, battery) = load_battery(&functions_toml(&["ex01"], "\"write\""));
    let white = TempDir::new().unwrap();
    compile_obj(white.path(), "ex01", PRINTF_WRITE_C);

    let (ok, events, _ctx) = run_symbols(&battery, Some((white, None)));
    assert!(!ok, "printf hors whitelist doit faire échouer l'étape");
    let checks = extract_checks(&events);
    let ko: Vec<&(String, bool, String)> = checks.iter().filter(|(_, ok, _)| !ok).collect();
    assert_eq!(ko.len(), 1, "un seul KO attendu : {checks:?}");
    assert_eq!(ko[0].0, "forbidden function");
    assert_eq!(ko[0].2, "forbidden function: printf");
    // write est whitelisté : jamais flaggé.
    assert!(
        !checks.iter().any(|(_, _, d)| d.contains("write")),
        "write flaggé malgré la whitelist : {checks:?}"
    );
    let (ok_ev, skipped, _) = step_finished(&events);
    assert!(!ok_ev && !skipped, "étape KO, non skipped");
}

#[test]
fn objet_whitelist_complete_ok() {
    if !cc_present() {
        eprintln!("cc absent du PATH : test skippé");
        return;
    }
    let (_bat_dir, battery) = load_battery(&functions_toml(&["ex01"], "\"printf\", \"write\""));
    let white = TempDir::new().unwrap();
    compile_obj(white.path(), "ex01", PRINTF_WRITE_C);

    let (ok, events, _ctx) = run_symbols(&battery, Some((white, None)));
    assert!(ok, "tout est whitelisté : {events:?}");
    let checks = extract_checks(&events);
    assert_eq!(checks.len(), 1, "un seul check OK : {checks:?}");
    assert!(checks[0].1, "le check unique est OK : {checks:?}");
    let (ok_ev, skipped, _) = step_finished(&events);
    assert!(ok_ev && !skipped);
}

#[test]
fn appels_inter_fonctions_du_rendu_jamais_flagges() {
    if !cc_present() {
        eprintln!("cc absent du PATH : test skippé");
        return;
    }
    // use_it.o référence helper, défini par helper.o : appel interne
    // au rendu, pas une dépendance externe.
    let (_bat_dir, battery) = load_battery(&functions_toml(&["helper", "use_it"], "\"write\""));
    let white = TempDir::new().unwrap();
    compile_obj(white.path(), "helper", HELPER_C);
    compile_obj(white.path(), "use_it", USE_IT_C);

    let (ok, events, _ctx) = run_symbols(&battery, Some((white, None)));
    assert!(
        ok,
        "un appel interne au rendu n'est pas interdit : {events:?}"
    );
    let checks = extract_checks(&events);
    assert!(
        checks.iter().all(|(_, ok, _)| *ok),
        "KO inattendu (helper flaggé ?) : {checks:?}"
    );
    assert!(
        !checks.iter().any(|(_, _, d)| d.contains("helper")),
        "helper ne doit jamais être flaggé : {checks:?}"
    );
}

#[test]
fn meme_symbole_interdit_deux_objets_dedupe() {
    if !cc_present() {
        eprintln!("cc absent du PATH : test skippé");
        return;
    }
    let (_bat_dir, battery) = load_battery(&functions_toml(&["part_a", "part_b"], "\"write\""));
    let white = TempDir::new().unwrap();
    compile_obj(white.path(), "part_a", PRINTF_A_C);
    compile_obj(white.path(), "part_b", PRINTF_B_C);

    let (ok, events, _ctx) = run_symbols(&battery, Some((white, None)));
    assert!(!ok);
    let checks = extract_checks(&events);
    let ko: Vec<&(String, bool, String)> = checks.iter().filter(|(_, ok, _)| !ok).collect();
    assert_eq!(
        ko.len(),
        1,
        "printf vu dans deux .o = un seul KO dédupliqué : {checks:?}"
    );
    assert_eq!(ko[0].2, "forbidden function: printf");
}

#[test]
fn objet_data_symbols_licites_ok() {
    if !cc_present() {
        eprintln!("cc absent du PATH : test skippé");
        return;
    }
    // fprintf whitelisté ; stderr/stdout référencés comme données.
    let (_bat_dir, battery) = load_battery(&functions_toml(&["ex01"], "\"fprintf\""));
    let white = TempDir::new().unwrap();
    compile_obj(white.path(), "ex01", FPRINTF_STREAMS_C);

    let (ok, events, _ctx) = run_symbols(&battery, Some((white, None)));
    assert!(
        ok,
        "les globals libc licites ne sont pas des fonctions interdites : {events:?}"
    );
    let checks = extract_checks(&events);
    assert!(
        checks.iter().all(|(_, ok, _)| *ok),
        "KO inattendu (stream flaggé ?) : {checks:?}"
    );
    // Garde-fou explicite : jamais de KO sur un symbole data.
    for sym in ["stderr", "stdout", "stdin", "environ"] {
        assert!(
            !checks.iter().any(|(_, _, d)| d.contains(sym)),
            "symbole data {sym} flaggé : {checks:?}"
        );
    }
}

#[test]
fn objet_reference_got_jamais_flaggee() {
    if !cc_present() {
        eprintln!("cc absent du PATH : test skippé");
        return;
    }
    // weak_fn est whitelistée (référence faible licite) : seul
    // `_GLOBAL_OFFSET_TABLE_` pourrait encore faire KO.
    let (_bat_dir, battery) = load_battery(&functions_toml(&["ex01"], "\"weak_fn\""));
    let white = TempDir::new().unwrap();
    compile_obj(white.path(), "ex01", WEAK_GOT_C);

    let (ok, events, _ctx) = run_symbols(&battery, Some((white, None)));
    assert!(
        ok,
        "une référence GOT n'est pas une fonction interdite : {events:?}"
    );
    let checks = extract_checks(&events);
    assert!(
        checks.iter().all(|(_, ok, _)| *ok),
        "KO inattendu (GOT flaggée ?) : {checks:?}"
    );
    assert!(
        !checks
            .iter()
            .any(|(_, _, d)| d.contains("_GLOBAL_OFFSET_TABLE_")),
        "_GLOBAL_OFFSET_TABLE_ flaggé : {checks:?}"
    );
}

#[test]
fn binaire_printf_hors_whitelist_ko() {
    if !cc_present() {
        eprintln!("cc absent du PATH : test skippé");
        return;
    }
    let (_bat_dir, battery) = load_battery(&binary_toml("\"write\""));
    let white = TempDir::new().unwrap();
    let prog = compile_prog(white.path(), HELLO_C);

    let (ok, events, _ctx) = run_symbols(&battery, Some((white, Some(prog))));
    assert!(!ok, "printf non whitelisté dans le binaire : KO");
    let checks = extract_checks(&events);
    let ko: Vec<&(String, bool, String)> = checks.iter().filter(|(_, ok, _)| !ok).collect();
    assert!(
        ko.iter().any(|(_, _, d)| d == "forbidden function: printf"),
        "KO « forbidden function: printf » absent : {checks:?}"
    );
    // Le nom est strippé de tout suffixe de version ELF
    // (`printf@@GLIBC_2.2.5` → `printf`) : aucun KO ne contient '@'.
    assert!(
        !ko.iter().any(|(_, _, d)| d.contains('@')),
        "suffixe de version non strippé : {ko:?}"
    );
}

#[test]
fn binaire_runtime_symbols_jamais_flagges() {
    if !cc_present() {
        eprintln!("cc absent du PATH : test skippé");
        return;
    }
    let (_bat_dir, battery) = load_battery(&binary_toml("\"printf\""));
    let white = TempDir::new().unwrap();
    let prog = compile_prog(white.path(), HELLO_C);

    let (ok, events, _ctx) = run_symbols(&battery, Some((white, Some(prog))));
    assert!(
        ok,
        "hello world + printf whitelisté doit passer : {events:?}"
    );
    let checks = extract_checks(&events);
    assert!(
        checks.iter().all(|(_, ok, _)| *ok),
        "KO inattendu (symbole runtime flaggé ?) : {checks:?}"
    );
    // Garde-fou explicite : le runtime C n'est JAMAIS flaggé.
    for sym in [
        "_start",
        "__libc_start_main",
        "__gmon_start__",
        "__cxa_finalize",
        "main",
    ] {
        assert!(
            !checks.iter().any(|(_, _, d)| d.contains(sym)),
            "symbole runtime {sym} flaggé : {checks:?}"
        );
    }
}

#[test]
fn binaire_corrompu_ko_explicite() {
    // Pas besoin de cc : l'artefact est écrit à la main.
    let (_bat_dir, battery) = load_battery(&binary_toml("\"write\""));
    let white = TempDir::new().unwrap();
    let prog = white.path().join("prog");
    fs::write(&prog, b"pas un ELF du tout").unwrap();

    let (ok, events, _ctx) = run_symbols(&battery, Some((white, Some(prog))));
    assert!(
        !ok,
        "un binaire corrompu est un KO explicite, jamais un panic"
    );
    let checks = extract_checks(&events);
    assert!(
        checks
            .iter()
            .any(|(n, ok, d)| n == "artefact" && !*ok && d.contains("prog")),
        "KO artefact illisible absent : {checks:?}"
    );
}

#[test]
fn objet_corrompu_ko_explicite() {
    let (_bat_dir, battery) = load_battery(&functions_toml(&["ex01"], "\"write\""));
    let white = TempDir::new().unwrap();
    fs::write(white.path().join("ex01.o"), b"\x7fELF casse").unwrap();

    let (ok, events, _ctx) = run_symbols(&battery, Some((white, None)));
    assert!(!ok, "un .o corrompu est un KO explicite, jamais un panic");
    let checks = extract_checks(&events);
    assert!(
        checks
            .iter()
            .any(|(n, ok, d)| n == "artefact" && !*ok && d.contains("ex01.o")),
        "KO artefact illisible absent : {checks:?}"
    );
}

/// Batterie Functions dont la delivery est un dossier (Rush1) —
/// whitelist paramétrée.
fn functions_toml_dossier(allowed: &str) -> String {
    format!(
        r#"[project]
name = "sym_dir"
type = "functions"
allowed_functions = [{allowed}]

[[task]]
name = "ex01"
delivery = "rush-1-1"
harness = "harness/ex01_main.c"
stdout = ""
"#
    )
}

#[test]
fn functions_delivery_dossier_chaque_o_du_dossier_scanne() {
    if !cc_present() {
        eprintln!("cc absent du PATH : test skippé");
        return;
    }
    let (_bat_dir, battery) = load_battery(&functions_toml_dossier("\"write\""));
    // Contrat build.rs étendu : les sources vivent dans le dossier de
    // delivery, leurs .o à la racine de la salle blanche, nommés d'après
    // le stem NAMESPACÉ de chaque fichier (« rush-1-1/part_a.c » →
    // « rush-1-1__part_a.o »). printf est dans part_b.
    let white = TempDir::new().unwrap();
    fs::create_dir(white.path().join("rush-1-1")).unwrap();
    fs::write(white.path().join("rush-1-1/part_a.c"), HELPER_C).unwrap();
    fs::write(white.path().join("rush-1-1/part_b.c"), PRINTF_B_C).unwrap();
    for (src, obj) in [
        ("part_a", "rush-1-1__part_a.o"),
        ("part_b", "rush-1-1__part_b.o"),
    ] {
        let status = Command::new("cc")
            .args([
                "-fno-builtin",
                "-c",
                &format!("rush-1-1/{src}.c"),
                "-o",
                obj,
            ])
            .current_dir(white.path())
            .status()
            .expect("lancement de cc impossible");
        assert!(status.success(), "cc -c {src}.c a échoué");
    }

    let (ok, events, _ctx) = run_symbols(&battery, Some((white, None)));
    assert!(!ok, "printf dans la delivery dossier doit être détecté");
    let checks = extract_checks(&events);
    assert!(
        checks
            .iter()
            .any(|(_, ok, d)| !*ok && d == "forbidden function: printf"),
        "KO « forbidden function: printf » absent : {checks:?}"
    );
}
