//! Tests de l'étape Build (Task 5) : salle blanche, make fclean/re,
//! cflags pénalisants mais non bloquants, compilation des deliveries
//! Functions.
//!
//! Pas de test de timeout ici (pas de test lent) : la mécanique de
//! timeout est déjà couverte par tests/exec.rs.

mod common;

use seeyou::battery::Battery;
use seeyou::engine::build;
use seeyou::engine::events::{Event, Step};
use seeyou::engine::PipelineContext;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use tempfile::TempDir;

/// Batterie Binary factice adossée à la fixture ok_project.
const BINARY_TOML: &str = r#"
[project]
name = "ok_project"
type = "binary"
binary = "ok_project"
"#;

/// Batterie Binary factice adossée à la fixture warn_project.
const WARN_TOML: &str = r#"
[project]
name = "warn_project"
type = "binary"
binary = "warn_project"
"#;

/// Batterie Binary exigeant -Wpedantic, absent du Makefile d'ok_project.
const PEDANTIC_TOML: &str = r#"
[project]
name = "ok_project"
type = "binary"
binary = "ok_project"
cflags = ["-Wall", "-Wextra", "-Werror", "-Wpedantic"]
"#;

/// Batterie Functions factice : une seule delivery, my_putstr.c.
const FUNCTIONS_TOML: &str = r#"
[project]
name = "mini_functions"
type = "functions"

[[task]]
name = "ex01"
delivery = "my_putstr.c"
harness = "harness/ex01_main.c"
stdout = ""
"#;

/// Un my_putstr qui compile proprement avec -Wall -Wextra -Werror
/// (valeur de retour de write utilisée : glibc le déclare __wur).
const MY_PUTSTR_OK: &str = r#"/*
** EPITECH PROJECT, 2026
** functions fixture
** File description:
** my_putstr
*/

#include <unistd.h>

void my_putstr(char const *str)
{
    while (*str != '\0') {
        if (write(1, str, 1) == -1)
            return;
        str++;
    }
}
"#;

/// Le même, syntaxiquement cassé.
const MY_PUTSTR_BROKEN: &str = r#"/*
** EPITECH PROJECT, 2026
** functions fixture
** File description:
** my_putstr cassé
*/

void my_putstr(char const *str)
{
    str = ;
}
"#;

/// Écrit `toml` dans un tempdir et charge la batterie.
/// Le TempDir est renvoyé pour garder `battery.root` valide.
fn load_battery(toml: &str) -> (TempDir, Battery) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("moulinette.toml");
    fs::write(&path, toml).unwrap();
    (dir, Battery::load(&path).unwrap())
}

/// Chemin d'une fixture versionnée.
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// Exécute l'étape build sur `target` ; renvoie le verdict, les
/// événements et le ctx (qui garde la salle blanche vivante pour les
/// assertions).
fn run_build(battery: &Battery, target: &Path) -> (bool, Vec<Event>, PipelineContext) {
    let mut ctx = common::test_ctx(battery, target);
    let (tx, rx) = mpsc::channel();
    let ok = build::run(&mut ctx, &tx);
    drop(tx);
    let events: Vec<Event> = rx.iter().collect();
    (ok, events, ctx)
}

/// Extrait les checks `(name, ok, detail)` du flux ; tout check doit
/// être estampillé Build.
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
                assert_eq!(*step, Step::Build, "check émis pour la mauvaise étape");
                Some((name.clone(), *ok, detail.clone()))
            }
            _ => None,
        })
        .collect()
}

/// Noms des fichiers à la racine de `dir`, triés.
fn root_entries(dir: &Path) -> Vec<String> {
    let mut entries: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    entries
}

#[test]
fn binary_ok_build_reussit_sans_salir_la_cible() {
    let (_bat_dir, battery) = load_battery(BINARY_TOML);
    let target = fixture("ok_project");
    let (ok, events, ctx) = run_build(&battery, &target);

    assert!(ok, "build KO sur ok_project : {events:?}");
    // L'étape est correctement encadrée.
    assert!(matches!(
        events.first(),
        Some(Event::StepStarted {
            step: Step::Build,
            ..
        })
    ));
    assert!(events.iter().any(|e| matches!(
        e,
        Event::StepFinished {
            step: Step::Build,
            ok: true,
            skipped: false,
            ..
        }
    )));

    // Le binaire produit est répertorié, dans la salle blanche.
    let build = ctx.build.as_ref().expect("ctx.build posé");
    let binary = build.binary.as_ref().expect("binaire produit répertorié");
    assert!(binary.is_file(), "binaire absent : {}", binary.display());
    assert_eq!(binary.file_name().unwrap(), "ok_project");
    assert!(
        binary.starts_with(build.dir.path()),
        "binaire hors de la salle blanche : {}",
        binary.display()
    );

    // La sortie de make est relayée en LogLine : les recettes sont
    // echoées, au moins la compilation de main.c.
    let mut saw_compile_echo = false;
    for e in &events {
        if let Event::LogLine { step, line } = e {
            assert_eq!(*step, Step::Build, "LogLine hors Build");
            saw_compile_echo |= line.contains("main.c");
        }
    }
    assert!(
        saw_compile_echo,
        "recette de compilation absente des LogLine : {events:?}"
    );

    // Les 3 cflags par défaut sont vérifiés OK dans le Makefile, et
    // make fclean / make re ont chacun leur check OK.
    let checks = extract_checks(&events);
    for flag in ["-Wall", "-Wextra", "-Werror"] {
        assert!(
            checks
                .iter()
                .any(|(name, ok, d)| name == "cflags" && *ok && d.contains(flag)),
            "cflag {flag} non vérifié OK : {checks:?}"
        );
    }
    for rule in ["fclean", "re"] {
        assert!(
            checks
                .iter()
                .any(|(name, ok, d)| name == "make" && *ok && d.contains(rule)),
            "check make {rule} OK absent : {checks:?}"
        );
    }

    // La cible n'est JAMAIS modifiée : ni .o ni binaire dans la fixture.
    assert_eq!(
        root_entries(&target),
        vec!["Makefile", "main.c"],
        "fixture salie par le build"
    );
}

#[test]
fn binary_warn_build_echoue_sur_werror() {
    let (_bat_dir, battery) = load_battery(WARN_TOML);
    let target = fixture("warn_project");
    let (ok, events, ctx) = run_build(&battery, &target);

    assert!(
        !ok,
        "-Werror + variable inutilisée doit faire échouer le build"
    );
    assert!(events.iter().any(|e| matches!(
        e,
        Event::StepFinished {
            step: Step::Build,
            ok: false,
            ..
        }
    )));
    let checks = extract_checks(&events);
    assert!(
        checks
            .iter()
            .any(|(name, ok, d)| name == "make" && !*ok && d.contains("build failed")),
        "CheckFinished KO 'build failed' absent : {checks:?}"
    );
    // L'extrait stderr montre la cause (LC_ALL=C : « error: » stable).
    assert!(
        checks
            .iter()
            .any(|(name, ok, d)| name == "make" && !*ok && d.contains("error")),
        "extrait stderr absent du KO : {checks:?}"
    );
    // Salle blanche posée mais pas de binaire répertorié.
    let build = ctx.build.as_ref().expect("ctx.build posé");
    assert!(build.binary.is_none());
    // La fixture reste intacte.
    assert_eq!(
        root_entries(&target),
        vec!["Makefile", "main.c"],
        "fixture salie par le build"
    );
}

#[test]
fn binary_cflag_manquant_penalise_sans_bloquer() {
    let (_bat_dir, battery) = load_battery(PEDANTIC_TOML);
    let (ok, events, ctx) = run_build(&battery, &fixture("ok_project"));

    assert!(
        ok,
        "un cflag manquant pénalise mais ne bloque pas : {events:?}"
    );
    let checks = extract_checks(&events);
    assert!(
        checks.iter().any(|(name, ok, d)| name == "cflags"
            && !*ok
            && d.contains("missing cflag: -Wpedantic")),
        "KO 'missing cflag: -Wpedantic' absent : {checks:?}"
    );
    // Les flags présents restent vérifiés OK.
    assert!(checks.iter().any(|(name, ok, _)| name == "cflags" && *ok));
    // Et le binaire est quand même produit et répertorié.
    assert!(ctx
        .build
        .as_ref()
        .and_then(|b| b.binary.as_ref())
        .is_some_and(|p| p.is_file()));
}

#[test]
fn binary_sans_makefile_ko_explicite() {
    let (_bat_dir, battery) = load_battery(BINARY_TOML);
    let target = TempDir::new().unwrap(); // repo vide : pas de Makefile

    let (ok, events, _ctx) = run_build(&battery, target.path());
    assert!(!ok);
    let checks = extract_checks(&events);
    assert!(
        checks
            .iter()
            .any(|(_, ok, d)| !*ok && d.contains("missing Makefile")),
        "KO 'missing Makefile' absent : {checks:?}"
    );
}

#[test]
fn functions_compile_ok_produit_les_objets() {
    let (_bat_dir, battery) = load_battery(FUNCTIONS_TOML);
    let target = TempDir::new().unwrap();
    fs::write(target.path().join("my_putstr.c"), MY_PUTSTR_OK).unwrap();

    let (ok, events, ctx) = run_build(&battery, target.path());
    assert!(ok, "build KO sur une delivery saine : {events:?}");
    let build = ctx.build.as_ref().expect("ctx.build posé");
    assert!(
        build.dir.path().join("my_putstr.o").is_file(),
        "my_putstr.o absent de la salle blanche"
    );
    let checks = extract_checks(&events);
    assert!(checks.iter().any(|(name, ok, _)| name == "compile" && *ok));
    // La cible n'est pas salie : le .o n'existe qu'en salle blanche.
    assert_eq!(root_entries(target.path()), vec!["my_putstr.c"]);
}

#[test]
fn functions_syntaxe_cassee_fait_echouer_l_etape() {
    let (_bat_dir, battery) = load_battery(FUNCTIONS_TOML);
    let target = TempDir::new().unwrap();
    fs::write(target.path().join("my_putstr.c"), MY_PUTSTR_BROKEN).unwrap();

    let (ok, events, _ctx) = run_build(&battery, target.path());
    assert!(!ok, "une delivery cassée doit faire échouer le build");
    let checks = extract_checks(&events);
    assert!(
        checks.iter().any(|(name, ok, d)| name == "compile"
            && !*ok
            && d.contains("compile failed: my_putstr.c")
            && d.contains("error")),
        "KO compile avec extrait stderr absent : {checks:?}"
    );
}
