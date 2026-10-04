//! CLI all-seeing-eye (Task 12).
//!
//! Trois modes :
//! - **défaut** : découvre la batterie du cwd ([`Battery::discover`])
//!   ou charge celle de `--battery <nom>`, lance le pipeline dans un
//!   thread et affiche le TUI live (`--no-tui` : lignes texte sur
//!   stdout, pensé CI, code de sortie 0/1) ;
//! - **`list`** : les batteries connues (config + embarquées) ;
//! - **`report`** : rouvre le dernier rapport (TUI de navigation, ou
//!   texte sur stdout avec `--no-tui`).
//!
//! Le thread pipeline est TOUJOURS enrobé de
//! [`std::panic::catch_unwind`] : un panic pipeline ne doit jamais
//! emporter le process — le sender est droppé dans le thread quoi
//! qu'il arrive (retour normal ou unwind), donc le front voit le
//! canal se fermer et affiche « pipeline interrompu » (Task 11) au
//! lieu d'un écran figé.

use crate::battery::model::ProjectType;
use crate::battery::{self, Battery};
use crate::engine::events::{Event, TestVerdict};
use crate::engine::{self, RunOpts};
use crate::norme::official;
use crate::report::{self, Report};
use crate::tui;
use anyhow::{bail, ensure, Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Instant;

/// Ligne de commande `all-seeing-eye`.
#[derive(Debug, Parser)]
#[command(
    name = "all-seeing-eye",
    version,
    about = "All-Seeing Eye — moulinette locale"
)]
pub struct Cli {
    /// Charge la batterie `<nom>` (config puis embarquées) au lieu de
    /// la découvrir dans le dossier courant.
    #[arg(long)]
    pub battery: Option<String>,
    /// Pas de TUI : events et rapport en texte sur stdout (CI) —
    /// acceptée aussi après une sous-commande.
    // `global = true` : `all-seeing-eye report --no-tui` doit marcher.
    #[arg(long, global = true)]
    pub no_tui: bool,
    /// Norme stricte : epiclang requis, étape Norme KO s'il est absent.
    #[arg(long)]
    pub strict_norme: bool,
    #[command(subcommand)]
    pub cmd: Option<Sub>,
}

/// Sous-commandes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum Sub {
    /// Liste les batteries connues (config + embarquées).
    List,
    /// Rouvre le dernier rapport (TUI ; texte avec --no-tui).
    Report,
}

impl Cli {
    /// Dispatch et exécution ; renvoie le code de sortie du process
    /// (0/1 — une `Err` vaut 1, cf. `main`).
    pub fn execute(&self) -> Result<i32> {
        match self.cmd {
            Some(Sub::List) => {
                cmd_list();
                Ok(0)
            }
            Some(Sub::Report) => cmd_report(self.no_tui),
            None => cmd_run(self),
        }
    }
}

/// Mode défaut : batterie (découverte dans le cwd ou `--battery`),
/// détection du compilateur, run TUI ou texte.
fn cmd_run(cli: &Cli) -> Result<i32> {
    let target = std::env::current_dir().context("répertoire courant illisible")?;
    let battery = match &cli.battery {
        Some(nom) => batterie_nommee(nom)?,
        None => Battery::discover(&target)
            .map_err(|e| e.context("astuce : `all-seeing-eye list` liste les batteries connues"))?,
    };
    let opts = detect_opts(cli.strict_norme);
    if cli.no_tui {
        run_texte(&battery, &target, &opts)
    } else {
        run_tui(&battery, &target, &opts)
    }
}

/// Options du run, détection compilateur incluse : la MÊME sonde que
/// la norme ([`official::epiclang_available`] — `epiclang --version`
/// spawnable). epiclang présent → il compile aussi ; sinon, cc.
fn detect_opts(strict_norme: bool) -> RunOpts {
    let use_epiclang = official::epiclang_available();
    RunOpts {
        strict_norme,
        use_epiclang,
        compiler: if use_epiclang {
            PathBuf::from("epiclang")
        } else {
            PathBuf::from("cc")
        },
    }
}

/// Lance le pipeline dans un thread et renvoie le receiver des events
/// plus le handle. `catch_unwind` enrobe TOUT le run : le sender vit
/// dans le thread et y est droppé dans tous les cas (retour normal
/// ou unwind) — le front voit donc toujours le canal se fermer, et
/// le process survit à un panic pipeline.
fn spawn_pipeline(
    battery: &Battery,
    target: &Path,
    opts: &RunOpts,
) -> (mpsc::Receiver<Event>, std::thread::JoinHandle<()>) {
    let (tx, rx) = mpsc::channel::<Event>();
    let battery = battery.clone();
    let target = target.to_path_buf();
    let opts = opts.clone();
    let handle = std::thread::spawn(move || {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            engine::run_pipeline(&battery, &target, tx, &opts);
        }));
    });
    (rx, handle)
}

/// Run avec TUI : le pipeline tourne dans son thread, le TUI consomme
/// les events sur le thread principal (raw mode crossterm). Au `q`
/// post-run, le rapport texte complet part sur stdout.
fn run_tui(battery: &Battery, target: &Path, opts: &RunOpts) -> Result<i32> {
    let started = Instant::now();
    let (rx, handle) = spawn_pipeline(battery, target, opts);
    let report = tui::run(rx, battery.project.name.clone(), started)?;
    // RunFinished reçu (sinon `tui::run` a déjà renvoyé une erreur) :
    // le thread pipeline est fini ou mort, join immédiat — jamais de
    // thread laissé en vol.
    let _ = handle.join();
    print!("{}", report::render_text(&report));
    Ok(exit_code(&report))
}

/// Run `--no-tui` : le compilateur détecté est logué en en-tête,
/// chaque event devient une ligne texte immédiate (cf.
/// [`affiche_event`]), le rapport complet termine la sortie. Code de
/// sortie CI via [`exit_code`]. Un pipeline mort sans verdict (panic)
/// est une erreur propre, pas un abort du process.
fn run_texte(battery: &Battery, target: &Path, opts: &RunOpts) -> Result<i32> {
    println!(
        "all-seeing-eye — batterie « {} » — compilateur : {}",
        battery.project.name,
        opts.compiler.display()
    );
    let (rx, handle) = spawn_pipeline(battery, target, opts);
    let mut verdict: Option<Report> = None;
    // Itérateur bloquant : se termine quand le sender (dans le thread
    // pipeline) est droppé — run fini OU panic.
    for ev in &rx {
        match ev {
            Event::RunFinished { report } => verdict = Some(*report),
            _ => affiche_event(&ev),
        }
    }
    let _ = handle.join();
    let Some(report) = verdict else {
        bail!("pipeline interrompu avant le verdict (panic ?) — aucun rapport");
    };
    print!("{}", report::render_text(&report));
    Ok(exit_code(&report))
}

/// Une ligne texte par event (mode `--no-tui`, pensé CI) :
/// `[etape] message` pour la progression, `✓/✗/💥/⏱ nom` pour les
/// tests, `file:line:col severity message (rule)` pour les fautes de
/// norme. `TestStarted` et `RunFinished` n'impriment rien : le verdict
/// du test suffit, et le rapport complet suit.
fn affiche_event(ev: &Event) {
    match ev {
        Event::StepStarted { step, label } => println!("[{}] {label}", step.name()),
        Event::LogLine { step, line } => println!("[{}] {line}", step.name()),
        Event::CheckFinished {
            step,
            name,
            ok,
            detail,
        } => {
            let statut = if *ok { "ok" } else { "KO" };
            if detail.is_empty() {
                println!("[{}] {statut} {name}", step.name());
            } else {
                println!("[{}] {statut} {name} — {detail}", step.name());
            }
        }
        Event::TestStarted { .. } => {}
        Event::TestFinished { name, result, .. } => {
            let icone = match result {
                TestVerdict::Passed => "✓",
                TestVerdict::Failed { .. } => "✗",
                TestVerdict::Crashed(_) => "💥",
                TestVerdict::Timeout => "⏱",
            };
            println!("  {icone} {name}");
        }
        Event::NormeFault(f) => println!(
            "{}:{}:{}: {} {} ({})",
            f.file.display(),
            f.line,
            f.col,
            f.severity.label(),
            f.message,
            f.rule
        ),
        Event::StepFinished {
            step,
            ok,
            skipped,
            summary,
        } => {
            let statut = if *skipped {
                "skipped"
            } else if *ok {
                "OK"
            } else {
                "KO"
            };
            println!("[{}] {statut} — {summary}", step.name());
        }
        Event::RunFinished { .. } => {}
    }
}

/// Code de sortie CI : 0 si score global 100.0 ET toutes les étapes
/// OK, 1 sinon. (Les étapes désactivées — symbols sans whitelist,
/// unit sans `tests_run_rule` — ont `ok: true` et ne bloquent pas.)
fn exit_code(report: &Report) -> i32 {
    if report.scores.global == 100.0 && report.steps.iter().all(|s| s.ok) {
        0
    } else {
        1
    }
}

/// `--battery <nom>` : `~/.config/all-seeing-eye/batteries/<nom>.toml`
/// d'abord, les embarquées ensuite (Task 13 — la fonction est vide
/// pour l'instant). Introuvable → erreur listant les batteries
/// connues.
fn batterie_nommee(nom: &str) -> Result<Battery> {
    // Un NOM, pas un chemin : on ne lit jamais ailleurs que dans le
    // dossier batteries de la config.
    ensure!(
        !nom.is_empty() && nom != "." && nom != ".." && !nom.contains(['/', '\\']),
        "nom de batterie invalide : « {nom} » (un nom de fichier, pas un chemin)"
    );
    if let Some(config) = dirs::config_dir() {
        let path = config
            .join("all-seeing-eye")
            .join("batteries")
            .join(format!("{nom}.toml"));
        if path.is_file() {
            return Battery::load(&path);
        }
    }
    // Embarquées : Task 13 branchera l'extraction sur disque (le
    // `root` doit exister en vrai) — la source est vide pour
    // l'instant, cette branche est donc encore morte.
    if battery::embedded_batteries()
        .iter()
        .any(|(em_nom, _)| *em_nom == nom)
    {
        bail!("batterie embarquée « {nom} » : chargement branché en Task 13");
    }
    let connues = batteries_connues();
    let mut detail = String::from("batteries connues :");
    if connues.is_empty() {
        detail.push_str(" aucune (ni config, ni embarquée)");
    } else {
        for b in &connues {
            detail.push_str(&format!("\n  - {}", b.ligne()));
        }
    }
    bail!("batterie « {nom} » introuvable — {detail}\n(astuce : `all-seeing-eye list`)")
}

/// Une batterie connue, pour `list` et les messages d'erreur.
struct BatterieConnue {
    nom: String,
    /// « binary » | « functions » ; « ? » si le TOML est illisible.
    kind: String,
    /// Résumé court (comptes de tâches/tests) ou l'erreur de lecture.
    description: String,
    /// Chemin du TOML, ou « embarquée ».
    origine: String,
}

impl BatterieConnue {
    /// Ligne d'affichage : `nom ⇥ type ⇥ description ⇥ (origine)`.
    fn ligne(&self) -> String {
        format!(
            "{}\t{}\t{}\t({})",
            self.nom, self.kind, self.description, self.origine
        )
    }

    /// Entrée d'une batterie chargée.
    fn de_battery(b: &Battery, origine: String) -> Self {
        let kind = match b.project.kind {
            ProjectType::Binary => "binary",
            ProjectType::Functions => "functions",
        };
        Self {
            nom: b.project.name.clone(),
            kind: kind.to_string(),
            description: description_courte(b),
            origine,
        }
    }
}

/// Toutes les batteries connues :
/// `~/.config/all-seeing-eye/batteries/*.toml` (triées par chemin ;
/// les illisibles restent listées avec leur erreur — jamais
/// silencieusement omises), puis les embarquées (Task 13).
fn batteries_connues() -> Vec<BatterieConnue> {
    let mut out = Vec::new();
    if let Some(config) = dirs::config_dir() {
        let dir = config.join("all-seeing-eye").join("batteries");
        let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
            .map(|rd| {
                rd.filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("toml"))
                    .collect()
            })
            .unwrap_or_default();
        paths.sort();
        for path in paths {
            let entree = match Battery::load(&path) {
                Ok(b) => BatterieConnue::de_battery(&b, path.display().to_string()),
                Err(e) => BatterieConnue {
                    nom: path
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.display().to_string()),
                    kind: "?".to_string(),
                    description: format!("illisible : {e:#}"),
                    origine: path.display().to_string(),
                },
            };
            out.push(entree);
        }
    }
    for (nom, contenu) in battery::embedded_batteries() {
        let entree = match toml::from_str::<Battery>(contenu) {
            Ok(b) => BatterieConnue::de_battery(&b, "embarquée".to_string()),
            Err(e) => BatterieConnue {
                nom: nom.to_string(),
                kind: "?".to_string(),
                description: format!("illisible : {e:#}"),
                origine: "embarquée".to_string(),
            },
        };
        out.push(entree);
    }
    out
}

/// Résumé court d'une batterie : compte de tâches (functions) ou de
/// tests fonctionnels (binary), règle `tests_run` signalée.
fn description_courte(b: &Battery) -> String {
    let mut d = match b.project.kind {
        ProjectType::Functions => format!("{} tâche{}", b.task.len(), pluriel(b.task.len())),
        ProjectType::Binary => format!(
            "{} test{} fonctionnel{}",
            b.functional_test.len(),
            pluriel(b.functional_test.len()),
            pluriel(b.functional_test.len())
        ),
    };
    if b.project.tests_run_rule {
        d.push_str(" + tests_run");
    }
    d
}

/// « s » au pluriel (au-delà de 1), rien au singulier.
fn pluriel(n: usize) -> &'static str {
    if n > 1 {
        "s"
    } else {
        ""
    }
}

/// `all-seeing-eye list` : une ligne par batterie connue.
fn cmd_list() {
    let connues = batteries_connues();
    if connues.is_empty() {
        println!(
            "aucune batterie connue — déposez des TOML dans \
             ~/.config/all-seeing-eye/batteries/"
        );
        return;
    }
    for b in &connues {
        println!("{}", b.ligne());
    }
}

/// `all-seeing-eye report` : recharge le dernier rapport
/// (`<data_dir>/all-seeing-eye/last.json`) — TUI de navigation, ou
/// texte sur stdout avec `--no-tui`. Erreurs claires si absent ou
/// corrompu.
fn cmd_report(no_tui: bool) -> Result<i32> {
    let path = dirs::data_dir()
        .context("aucun data dir (ni XDG_DATA_HOME ni HOME)")?
        .join("all-seeing-eye")
        .join("last.json");
    let contenu = fs::read_to_string(&path).with_context(|| {
        format!(
            "rapport introuvable : {} — lancez d'abord `all-seeing-eye`",
            path.display()
        )
    })?;
    let report: Report = serde_json::from_str(&contenu)
        .with_context(|| format!("rapport corrompu : {}", path.display()))?;
    if no_tui {
        print!("{}", report::render_text(&report));
    } else {
        tui::browse(report)?;
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::Scores;
    use crate::report::StepReport;

    /// Rapport minimal au score global et à l'état d'étape voulus.
    fn rapport(global: f64, etapes_ok: bool) -> Report {
        Report {
            project: "x".to_string(),
            steps: vec![StepReport {
                step: "prelim".to_string(),
                ok: etapes_ok,
                skipped: false,
                summary: String::new(),
                checks: Vec::new(),
            }],
            tests: Vec::new(),
            norme: Vec::new(),
            scores: Scores {
                global,
                ..Scores::default()
            },
            duration_secs: 0.0,
        }
    }

    #[test]
    fn exit_code_0_seulement_si_100_et_toutes_etapes_ok() {
        assert_eq!(exit_code(&rapport(100.0, true)), 0);
        assert_eq!(exit_code(&rapport(99.9, true)), 1);
        assert_eq!(exit_code(&rapport(100.0, false)), 1);
        assert_eq!(exit_code(&rapport(0.0, true)), 1);
    }

    #[test]
    fn description_courte_compte_taches_tests_et_tests_run() {
        let functions: Battery = toml::from_str(
            "[project]\nname = \"f\"\ntype = \"functions\"\n\n\
             [[task]]\nname = \"t\"\ndelivery = \"a.c\"\nharness = \"h.c\"\nstdout = \"\"\n",
        )
        .unwrap();
        assert_eq!(description_courte(&functions), "1 tâche");
        let binary: Battery = toml::from_str(
            "[project]\nname = \"b\"\ntype = \"binary\"\nbinary = \"b\"\n\
             tests_run_rule = true\n\n\
             [[functional_test]]\nname = \"t1\"\nstdout = \"\"\n\n\
             [[functional_test]]\nname = \"t2\"\nstdout = \"\"\n",
        )
        .unwrap();
        assert_eq!(
            description_courte(&binary),
            "2 tests fonctionnels + tests_run"
        );
    }

    #[test]
    fn batterie_nommee_rejette_les_chemins() {
        // Rejet AVANT tout accès disque : aucune de ces valeurs ne
        // doit lire hors du dossier batteries de la config.
        for nom in ["", ".", "..", "a/b", "a\\b"] {
            assert!(batterie_nommee(nom).is_err(), "accepté : « {nom} »");
        }
    }
}
