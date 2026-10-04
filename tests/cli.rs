//! Tests E2E du CLI (Task 12).
//!
//! Couvre ce qui est testable sans terminal : le parsing clap
//! (`Cli::try_parse_from`), le mode `--no-tui` sur le binaire compilé
//! (`CARGO_BIN_EXE_all-seeing-eye`), `list` et `report`. Les variables
//! XDG sont passées DIRECTEMENT dans l'env du process enfant (jamais
//! dans le process de test : pas de mutation globale, pas de
//! sérialisation sur XDG_MUTEX) — l'enfant hérite exactement ce qu'on
//! lui donne.
//!
//! Les fixtures C sont banana-clean (en-tête Epitech, aucun prototype
//! dans les `.c`) : sur une machine AVEC epiclang, la détection du CLI
//! le choisit comme compilateur, et banana + `-Werror` transforment
//! toute faute de norme en erreur de compilation — une fixture sale
//! ferait échouer le build pour la mauvaise raison.
//!
//! NB : ces tests E2E sont Linux-only de fait — `dirs::config_dir`
//! ignore `XDG_CONFIG_HOME` sur macOS (l'isolement des fixtures n'y
//! fonctionnerait pas), et l'outil est de toute façon Linux-scopé.

use all_seeing_eye::cli::{Cli, Sub};
use clap::Parser;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

/// Batterie Functions minimale chargée via `--battery e2e_cli` :
/// une task ex01 (delivery compilée avec son harness, stdout vide).
const BATTERIE_TOML: &str = r#"
[project]
name = "e2e_cli"
type = "functions"

[[task]]
name = "ex01"
delivery = "my_putchar.c"
harness = "harness/ex01_main.c"
stdout = ""
"#;

/// Harness banana-clean : en-tête Epitech, main nu (un prototype dans
/// un `.c` serait une C-H1 — fatale au build sous epiclang -Werror).
/// `concat!` et NON des continuations `\` : celles-ci mangent
/// l'indentation en début de ligne (C-L2 sous banana).
const HARNESS: &str = concat!(
    "/*\n",
    "** EPITECH PROJECT, 2026\n",
    "** e2e_cli\n",
    "** File description:\n",
    "** ex01 main\n",
    "*/\n",
    "\n",
    "int main(void)\n",
    "{\n",
    "    return 0;\n",
    "}\n",
);

/// Delivery correcte, banana-clean : en-tête + définition (la
/// déclaration seule serait une C-H1 ; `(void)c` évite le
/// -Wunused-parameter).
const DELIVERY_OK: &str = concat!(
    "/*\n",
    "** EPITECH PROJECT, 2026\n",
    "** e2e_cli\n",
    "** File description:\n",
    "** my_putchar\n",
    "*/\n",
    "\n",
    "void my_putchar(char c)\n",
    "{\n",
    "    (void)c;\n",
    "}\n",
);

/// Delivery cassée : accolade jamais fermée — la compilation échoue
/// sous cc comme sous epiclang.
const DELIVERY_KO: &str = "void my_putchar(char c) {\n";

/// Installe la batterie `e2e_cli` (TOML + harness) dans le config dir
/// isolé (`<config>/all-seeing-eye/batteries/`).
fn installe_batterie(config: &Path) {
    let dir = config.join("all-seeing-eye").join("batteries");
    fs::create_dir_all(dir.join("harness")).unwrap();
    fs::write(dir.join("harness/ex01_main.c"), HARNESS).unwrap();
    fs::write(dir.join("e2e_cli.toml"), BATTERIE_TOML).unwrap();
}

/// Le binaire compilé par cargo pour ces tests d'intégration.
fn binaire() -> Command {
    Command::new(env!("CARGO_BIN_EXE_all-seeing-eye"))
}

/// Contexte d'un run E2E : config (batteries), data (rapports) et
/// rendu (cwd de l'enfant) isolés dans des tempdirs.
struct Fixture {
    config: TempDir,
    data: TempDir,
    rendu: TempDir,
}

impl Fixture {
    /// Config avec la batterie e2e_cli + rendu contenant `delivery`.
    fn avec_batterie(delivery: &str) -> Fixture {
        let f = Fixture {
            config: TempDir::new().unwrap(),
            data: TempDir::new().unwrap(),
            rendu: TempDir::new().unwrap(),
        };
        installe_batterie(f.config.path());
        fs::write(f.rendu.path().join("my_putchar.c"), delivery).unwrap();
        f
    }

    /// Commande prête : env XDG isolées, cwd = rendu.
    fn commande(&self) -> Command {
        let mut c = binaire();
        c.current_dir(self.rendu.path())
            .env("XDG_CONFIG_HOME", self.config.path())
            .env("XDG_DATA_HOME", self.data.path());
        c
    }

    /// Run `--no-tui --battery e2e_cli` complet.
    fn run_no_tui(&self) -> Output {
        self.commande()
            .args(["--no-tui", "--battery", "e2e_cli"])
            .output()
            .unwrap()
    }
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

// ── Parsing clap ─────────────────────────────────────────────────

#[test]
fn parse_defaut_aucune_option_ni_sous_commande() {
    let cli = Cli::try_parse_from(["all-seeing-eye"]).unwrap();
    assert_eq!(cli.battery, None);
    assert!(!cli.no_tui);
    assert!(!cli.strict_norme);
    assert_eq!(cli.cmd, None);
}

#[test]
fn parse_options_du_run() {
    let cli = Cli::try_parse_from([
        "all-seeing-eye",
        "--battery",
        "cpool_day03",
        "--no-tui",
        "--strict-norme",
    ])
    .unwrap();
    assert_eq!(cli.battery.as_deref(), Some("cpool_day03"));
    assert!(cli.no_tui);
    assert!(cli.strict_norme);
    assert_eq!(cli.cmd, None);
}

#[test]
fn parse_sous_commandes() {
    let cli = Cli::try_parse_from(["all-seeing-eye", "list"]).unwrap();
    assert_eq!(cli.cmd, Some(Sub::List));

    // --no-tui est global : acceptée APRÈS la sous-commande
    // (`all-seeing-eye report --no-tui` doit marcher).
    let cli = Cli::try_parse_from(["all-seeing-eye", "report", "--no-tui"]).unwrap();
    assert_eq!(cli.cmd, Some(Sub::Report));
    assert!(cli.no_tui);
}

#[test]
fn parse_option_inconnue_refusee() {
    assert!(Cli::try_parse_from(["all-seeing-eye", "--nope"]).is_err());
    assert!(Cli::try_parse_from(["all-seeing-eye", "nope"]).is_err());
}

// ── E2E --no-tui sur le binaire compilé ──────────────────────────

#[test]
fn no_tui_delivery_correcte_exit_0_score_et_rapport() {
    let f = Fixture::avec_batterie(DELIVERY_OK);
    let out = f.run_no_tui();
    let stdout = stdout(&out);
    assert!(
        out.status.success(),
        "exit attendu 0, reçu {}\nstdout:\n{stdout}\nstderr:\n{}",
        out.status,
        stderr(&out)
    );
    assert!(stdout.contains("✓ ex01"), "test ex01 absent\n{stdout}");
    // Le compilateur choisi est visible : en-tête ET LogLine de la
    // première étape (prelim).
    assert!(
        stdout.contains("compilateur :"),
        "compilateur absent de l'en-tête\n{stdout}"
    );
    assert!(
        stdout.contains("[prelim] compilateur :"),
        "LogLine compilateur absente de prelim\n{stdout}"
    );
    assert!(
        stdout.contains("SCORE GLOBAL : 100.0%"),
        "score absent\n{stdout}"
    );
    // Le verdict a sauvegardé dans le data dir ISOLÉ, jamais dans
    // celui de l'utilisateur.
    assert!(
        f.data.path().join("all-seeing-eye/last.json").is_file(),
        "last.json absent du data dir isolé"
    );
}

#[test]
fn no_tui_delivery_cassee_exit_1() {
    let f = Fixture::avec_batterie(DELIVERY_KO);
    let out = f.run_no_tui();
    let stdout = stdout(&out);
    assert_eq!(
        out.status.code(),
        Some(1),
        "exit attendu 1\nstdout:\n{stdout}\nstderr:\n{}",
        stderr(&out)
    );
    assert!(
        stdout.contains("SCORE GLOBAL : 0.0%"),
        "score nul attendu (build KO → functional vide)\n{stdout}"
    );
}

#[test]
fn list_affiche_la_batterie_du_config_isole() {
    let f = Fixture::avec_batterie(DELIVERY_OK);
    let out = f.commande().arg("list").output().unwrap();
    let stdout = stdout(&out);
    assert!(out.status.success(), "list KO : {}", stderr(&out));
    assert!(stdout.contains("e2e_cli"), "nom absent\n{stdout}");
    assert!(stdout.contains("functions"), "type absent\n{stdout}");
    assert!(
        stdout.contains("1 tâche"),
        "description courte absente\n{stdout}"
    );
}

#[test]
fn report_sans_last_json_erreur_claire() {
    let f = Fixture::avec_batterie(DELIVERY_OK);
    let out = f.commande().args(["report", "--no-tui"]).output().unwrap();
    assert!(
        !out.status.success(),
        "report sans last.json doit échouer\nstdout:\n{}",
        stdout(&out)
    );
    let stderr = stderr(&out);
    assert!(stderr.contains("last.json"), "message peu clair : {stderr}");
}

#[test]
fn report_no_tui_relit_le_dernier_run() {
    let f = Fixture::avec_batterie(DELIVERY_OK);
    let run = f.run_no_tui();
    assert!(run.status.success(), "run KO : {}", stderr(&run));
    let out = f.commande().args(["report", "--no-tui"]).output().unwrap();
    let stdout = stdout(&out);
    assert!(out.status.success(), "report KO : {}", stderr(&out));
    assert!(
        stdout.contains("SCORE GLOBAL : 100.0%"),
        "score absent\n{stdout}"
    );
}

#[test]
fn batterie_inconnue_erreur_listant_les_connues() {
    let f = Fixture::avec_batterie(DELIVERY_OK);
    let out = f
        .commande()
        .args(["--no-tui", "--battery", "inexistante"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = stderr(&out);
    assert!(stderr.contains("inexistante"), "nom absent : {stderr}");
    assert!(
        stderr.contains("e2e_cli"),
        "les batteries connues doivent être listées : {stderr}"
    );
}

#[test]
fn sans_compilateur_erreur_claire_au_demarrage() {
    let f = Fixture::avec_batterie(DELIVERY_OK);
    // PATH vide de tout compilateur : la détection (epiclang → cc →
    // gcc) doit échouer AVANT de lancer le pipeline, avec un message
    // qui nomme les trois.
    let out = f
        .commande()
        .args(["--no-tui", "--battery", "e2e_cli"])
        .env("PATH", "/nonexistent")
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "sans compilateur le run doit échouer\nstdout:\n{}",
        stdout(&out)
    );
    let stderr = stderr(&out);
    assert!(
        stderr.contains("aucun compilateur C trouvé")
            && stderr.contains("epiclang")
            && stderr.contains("cc")
            && stderr.contains("gcc"),
        "message peu clair : {stderr}"
    );
}

#[test]
fn list_aplatit_les_erreurs_toml_multilignes() {
    let f = Fixture::avec_batterie(DELIVERY_OK);
    // Une batterie cassée : l'erreur TOML est multi-lignes (contexte
    // ligne/colonne) — `list` doit rester à UNE ligne par batterie.
    fs::write(
        f.config.path().join("all-seeing-eye/batteries/cassee.toml"),
        "[[[pas du toml",
    )
    .unwrap();
    let out = f.commande().arg("list").output().unwrap();
    let stdout = stdout(&out);
    assert!(out.status.success(), "list KO : {}", stderr(&out));
    let lignes: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lignes.len(),
        2,
        "une ligne par batterie attendue (cassee + e2e_cli)\n{stdout}"
    );
    let cassee = lignes
        .iter()
        .find(|l| l.starts_with("cassee"))
        .expect("batterie cassée absente de la liste");
    assert!(
        cassee.contains("illisible"),
        "erreur non signalée : {cassee}"
    );
}
