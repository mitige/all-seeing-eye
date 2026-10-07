//! Tests de l'étape Build (Task 5) : salle blanche, make fclean/re,
//! cflags pénalisants mais non bloquants, compilation des deliveries
//! Functions.
//!
//! Pas de test de timeout ici (pas de test lent) : la mécanique de
//! timeout est déjà couverte par tests/exec.rs.

mod common;

use all_seeing_eye::battery::Battery;
use all_seeing_eye::engine::build;
use all_seeing_eye::engine::events::{Event, Step};
use all_seeing_eye::engine::PipelineContext;
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

/// Listing récursif relatif de `dir` (« type chemin »), trié — pour
/// vérifier que la cible est strictement intacte après le build.
fn listing(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap().flatten() {
            let rel = e.path().strip_prefix(dir).unwrap().to_path_buf();
            let ft = e.file_type().unwrap();
            let tag = if ft.is_dir() {
                "d"
            } else if ft.is_symlink() {
                "l"
            } else {
                "f"
            };
            out.push(format!("{tag} {}", rel.display()));
            if ft.is_dir() {
                stack.push(e.path());
            }
        }
    }
    out.sort();
    out
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
    // Le résumé ne masque pas les pénalités non bloquantes.
    let summary = events
        .iter()
        .find_map(|e| match e {
            Event::StepFinished {
                step: Step::Build,
                summary,
                ..
            } => Some(summary.clone()),
            _ => None,
        })
        .expect("StepFinished Build émis");
    assert!(
        summary.starts_with("build ok") && summary.contains("1 en échec"),
        "résumé masquant la pénalité cflags : {summary:?}"
    );
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

/// Batterie Functions : deux deliveries dont le stem (nom du `.o`
/// produit) est identique — collision garantie en salle blanche.
const COLLISION_TOML: &str = r#"
[project]
name = "collision"
type = "functions"

[[task]]
name = "ex01"
delivery = "a/foo.c"
harness = "harness/ex01_main.c"
stdout = ""

[[task]]
name = "ex02"
delivery = "b/foo.c"
harness = "harness/ex02_main.c"
stdout = ""
"#;

#[test]
fn functions_stems_en_collision_ko_explicite() {
    let (_bat_dir, battery) = load_battery(COLLISION_TOML);
    let target = TempDir::new().unwrap();
    fs::create_dir(target.path().join("a")).unwrap();
    fs::create_dir(target.path().join("b")).unwrap();
    fs::write(
        target.path().join("a/foo.c"),
        "int foo_a(void)\n{\n    return 1;\n}\n",
    )
    .unwrap();
    fs::write(
        target.path().join("b/foo.c"),
        "int foo_b(void)\n{\n    return 2;\n}\n",
    )
    .unwrap();

    let (ok, events, _ctx) = run_build(&battery, target.path());
    assert!(!ok, "deux deliveries de même stem doivent être KO");
    let checks = extract_checks(&events);
    assert!(
        checks.iter().any(|(name, ok, d)| name == "compile"
            && !*ok
            && d.contains("collision")
            && d.contains("a/foo.c")
            && d.contains("b/foo.c")),
        "KO 'collision de .o' absent ou muet : {checks:?}"
    );
}

/// Batterie Functions : delivery keep.c — le vrai fichier du rendu
/// « sale » du test de filtrage.
const KEEP_TOML: &str = r#"
[project]
name = "filtrage"
type = "functions"

[[task]]
name = "ex01"
delivery = "keep.c"
harness = "harness/ex01_main.c"
stdout = ""
"#;

/// Un keep.c qui compile proprement avec -Wall -Wextra -Werror.
const KEEP_C: &str = "int keep_me(void)\n{\n    return 42;\n}\n";

#[test]
fn salle_blanche_filtre_les_indesirables_cible_intacte() {
    let (_bat_dir, battery) = load_battery(KEEP_TOML);
    let target = TempDir::new().unwrap();
    let t = target.path();
    // Le vrai fichier à garder, compilable.
    fs::write(t.join("keep.c"), KEEP_C).unwrap();
    // Les indésirables attendus, racine et sous-dossier : .git, *.o,
    // *~, #*#, *.swp, symlink.
    fs::create_dir(t.join(".git")).unwrap();
    fs::write(t.join(".git/config"), "[core]\n").unwrap();
    fs::write(t.join("phantom.o"), b"\0").unwrap();
    fs::write(t.join("main.c~"), "x").unwrap();
    fs::write(t.join("#main.c#"), "x").unwrap();
    fs::write(t.join(".main.c.swp"), "x").unwrap();
    fs::create_dir(t.join("sub")).unwrap();
    fs::write(t.join("sub/deep.o"), b"\0").unwrap();
    fs::write(t.join("sub/#deep#"), "x").unwrap();
    std::os::unix::fs::symlink("keep.c", t.join("lien")).unwrap();

    let avant = listing(t);
    let (ok, _events, ctx) = run_build(&battery, t);
    assert!(ok, "build KO sur un rendu sale mais compilable");

    let white = ctx
        .build
        .as_ref()
        .expect("ctx.build posé")
        .dir
        .path()
        .to_path_buf();
    // keep.c est copié et compilé.
    assert!(white.join("keep.c").is_file(), "keep.c non copié");
    assert!(white.join("keep.o").is_file(), "keep.o non produit");
    // Aucun indésirable ne traverse (symlink_metadata ne suit pas les
    // liens : un symlink copié serait détecté).
    for rel in [
        ".git",
        "phantom.o",
        "main.c~",
        "#main.c#",
        ".main.c.swp",
        "lien",
        "sub/deep.o",
        "sub/#deep#",
    ] {
        assert!(
            fs::symlink_metadata(white.join(rel)).is_err(),
            "{rel} ne devrait pas être en salle blanche"
        );
    }
    // sub/ traverse (c'est un dossier) mais vide de ses indésirables.
    assert!(white.join("sub").is_dir());
    assert!(
        root_entries(&white.join("sub")).is_empty(),
        "sub/ devrait être vide en salle blanche"
    );
    // La cible est strictement intacte.
    assert_eq!(listing(t), avant, "la cible a été modifiée");
}

#[test]
fn binary_fclean_en_echec_re_jamais_tente() {
    let (_bat_dir, battery) = load_battery(BINARY_TOML);
    let target = TempDir::new().unwrap();
    fs::write(
        target.path().join("Makefile"),
        "fclean:\n\tfalse\n\nre:\n\t@echo MARQUEUR_RE_EXECUTE\n",
    )
    .unwrap();

    let (ok, events, _ctx) = run_build(&battery, target.path());
    assert!(!ok, "fclean KO doit faire échouer le build");
    let checks = extract_checks(&events);
    // Un seul check make : fclean, KO — re n'est jamais tenté.
    let makes: Vec<_> = checks.iter().filter(|(n, _, _)| n == "make").collect();
    assert_eq!(makes.len(), 1, "plusieurs règles make tentées : {makes:?}");
    assert!(
        !makes[0].1 && makes[0].2.contains("make fclean"),
        "le seul check make devrait être le KO de fclean : {makes:?}"
    );
    // Jamais de LogLine de la règle re.
    assert!(
        !events.iter().any(
            |e| matches!(e, Event::LogLine { line, .. } if line.contains("MARQUEUR_RE_EXECUTE"))
        ),
        "make re tenté malgré l'échec de fclean : {events:?}"
    );
}

/// Batterie Functions : delivery dans un sous-dossier.
const SUBDIR_TOML: &str = r#"
[project]
name = "subdir"
type = "functions"

[[task]]
name = "ex01"
delivery = "src/my_fn.c"
harness = "harness/ex01_main.c"
stdout = ""
"#;

#[test]
fn functions_delivery_en_sous_dossier_objet_a_la_racine() {
    let (_bat_dir, battery) = load_battery(SUBDIR_TOML);
    let target = TempDir::new().unwrap();
    fs::create_dir(target.path().join("src")).unwrap();
    fs::write(
        target.path().join("src/my_fn.c"),
        "int my_fn(void)\n{\n    return 7;\n}\n",
    )
    .unwrap();

    let (ok, _events, ctx) = run_build(&battery, target.path());
    assert!(ok, "delivery en sous-dossier doit compiler");
    let white = ctx
        .build
        .as_ref()
        .expect("ctx.build posé")
        .dir
        .path()
        .to_path_buf();
    // Contrat Task 7 : le .o est à la racine de la salle blanche,
    // nommé d'après le stem de la delivery.
    assert!(
        white.join("my_fn.o").is_file(),
        "my_fn.o absent de la racine : {:?}",
        root_entries(&white)
    );
    assert!(white.join("src/my_fn.c").is_file(), "delivery non copiée");
}

#[test]
fn binary_bad_makefile_build_ko() {
    let (_bat_dir, battery) = load_battery(BINARY_TOML);
    let (ok, events, _ctx) = run_build(&battery, &fixture("bad_makefile"));

    assert!(!ok, "un Makefile invalide doit faire échouer le build");
    let checks = extract_checks(&events);
    // Le Makefile ne parse pas : make échoue (dès fclean).
    assert!(
        checks.iter().any(|(name, ok, _)| name == "make" && !*ok),
        "check make KO absent : {checks:?}"
    );
}

// ----------------------------------------------------------------
// Extensions : delivery dossier (Rush1 : rush-1-1/*), [build]
// pre_commands et command (Rush2)
// ----------------------------------------------------------------

/// Batterie Functions dont la delivery est un dossier.
const RUSH_TOML: &str = r#"
[project]
name = "rush1"
type = "functions"

[[task]]
name = "ex01"
delivery = "rush-1-1"
harness = "harness/ex01_main.c"
stdout = ""
"#;

#[test]
fn functions_delivery_dossier_compile_chaque_c_en_o_tries() {
    let (_bat_dir, battery) = load_battery(RUSH_TOML);
    let target = TempDir::new().unwrap();
    let dir = target.path().join("rush-1-1");
    fs::create_dir(&dir).unwrap();
    // Écrits dans le désordre : la compilation doit être triée par nom.
    fs::write(
        dir.join("b_part.c"),
        "int b_part(void)\n{\n    return 2;\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("a_part.c"),
        "int a_part(void)\n{\n    return 1;\n}\n",
    )
    .unwrap();
    fs::write(dir.join("notes.txt"), "pas un source\n").unwrap();

    let (ok, events, ctx) = run_build(&battery, target.path());
    assert!(ok, "delivery dossier saine : build OK : {events:?}");
    let white = ctx
        .build
        .as_ref()
        .expect("ctx.build posé")
        .dir
        .path()
        .to_path_buf();
    // Contrat : un .o par source du dossier, à la racine, nommé d'après
    // le stem du fichier.
    for obj in ["a_part.o", "b_part.o"] {
        assert!(
            white.join(obj).is_file(),
            "{obj} absent de la salle blanche : {:?}",
            root_entries(&white)
        );
    }
    // Un check compile OK par source, dans l'ordre trié.
    let checks = extract_checks(&events);
    let compiles: Vec<&str> = checks
        .iter()
        .filter(|(n, ok, _)| n == "compile" && *ok)
        .map(|(_, _, d)| d.as_str())
        .collect();
    assert_eq!(
        compiles,
        vec!["rush-1-1/a_part.c ok", "rush-1-1/b_part.c ok"],
        "sources du dossier compilées triées : {compiles:?}"
    );
    // La cible n'est pas salie.
    assert_eq!(
        root_entries(&dir),
        vec!["a_part.c", "b_part.c", "notes.txt"],
        "dossier de delivery sali"
    );
}

#[test]
fn functions_delivery_dossier_sans_c_ko_explicite() {
    let (_bat_dir, battery) = load_battery(RUSH_TOML);
    let target = TempDir::new().unwrap();
    let dir = target.path().join("rush-1-1");
    fs::create_dir(&dir).unwrap();
    fs::write(dir.join("README.md"), "rien\n").unwrap();

    let (ok, events, _ctx) = run_build(&battery, target.path());
    assert!(!ok, "un dossier sans .c ne compile pas");
    let checks = extract_checks(&events);
    assert!(
        checks
            .iter()
            .any(|(n, ok, d)| n == "compile" && !*ok && d.contains("rush-1-1") && d.contains(".c")),
        "KO clair « dossier sans .c » absent : {checks:?}"
    );
}

/// Batterie Functions : une delivery dossier et une delivery fichier
/// dont un stem est en collision (rush-1-1/foo.c et foo.c).
const COLLISION_DIR_TOML: &str = r#"
[project]
name = "collision_dir"
type = "functions"

[[task]]
name = "ex01"
delivery = "rush-1-1"
harness = "harness/ex01_main.c"
stdout = ""

[[task]]
name = "ex02"
delivery = "foo.c"
harness = "harness/ex02_main.c"
stdout = ""
"#;

#[test]
fn functions_delivery_dossier_collision_de_stem_ko() {
    let (_bat_dir, battery) = load_battery(COLLISION_DIR_TOML);
    let target = TempDir::new().unwrap();
    fs::create_dir(target.path().join("rush-1-1")).unwrap();
    fs::write(
        target.path().join("rush-1-1/foo.c"),
        "int foo_a(void)\n{\n    return 1;\n}\n",
    )
    .unwrap();
    fs::write(
        target.path().join("foo.c"),
        "int foo_b(void)\n{\n    return 2;\n}\n",
    )
    .unwrap();

    let (ok, events, _ctx) = run_build(&battery, target.path());
    assert!(!ok, "stems en collision via un dossier : KO attendu");
    let checks = extract_checks(&events);
    assert!(
        checks.iter().any(|(n, ok, d)| n == "compile"
            && !*ok
            && d.contains("collision")
            && d.contains("rush-1-1/foo.c")
            && d.contains("foo.c")),
        "KO 'collision de .o' (dossier) absent ou muet : {checks:?}"
    );
}

/// Batterie Rush2-like : la lib est construite par une pre_command,
/// puis le binaire par une commande custom qui REMPLACE make.
const RUSH2_TOML: &str = r#"
[project]
name = "rush2"
type = "binary"
binary = "rush2"

[build]
pre_commands = ["cc -c lib.c -o lib.o && ar rcs libmini.a lib.o"]
command = "cc -o rush2 main.c -L. -lmini"
"#;

/// main.c appelant lib_fn (définie dans lib.c).
const RUSH2_MAIN: &str =
    "int lib_fn(void);\n\nint main(void)\n{\n    return lib_fn() == 42 ? 0 : 1;\n}\n";

/// lib.c définissant lib_fn.
const RUSH2_LIB: &str = "int lib_fn(void)\n{\n    return 42;\n}\n";

#[test]
fn binary_build_command_et_pre_commands_remplacent_make() {
    let (_bat_dir, battery) = load_battery(RUSH2_TOML);
    let target = TempDir::new().unwrap();
    // Pas de Makefile : make fclean/re échouerait, la commande custom
    // doit être le seul mécanisme de build.
    fs::write(target.path().join("main.c"), RUSH2_MAIN).unwrap();
    fs::write(target.path().join("lib.c"), RUSH2_LIB).unwrap();

    let (ok, events, ctx) = run_build(&battery, target.path());
    assert!(ok, "build custom KO : {events:?}");
    // Le binaire produit par la commande est répertorié, en salle
    // blanche.
    let build = ctx.build.as_ref().expect("ctx.build posé");
    let binary = build.binary.as_ref().expect("binaire répertorié");
    assert!(binary.is_file(), "rush2 absent : {}", binary.display());
    assert!(binary.starts_with(build.dir.path()));
    let checks = extract_checks(&events);
    assert!(
        checks.iter().any(|(n, ok, d)| n == "pre_command"
            && *ok
            && d.contains("pre_command ok: cc -c lib.c")),
        "check pre_command OK absent : {checks:?}"
    );
    assert!(
        checks
            .iter()
            .any(|(n, ok, d)| n == "build" && *ok && d.contains("build ok: cc -o rush2 main.c")),
        "check build OK absent : {checks:?}"
    );
    // Ni make ni cflags sans Makefile.
    assert!(
        checks
            .iter()
            .all(|(n, _, _)| n != "make" && n != "cflags" && n != "makefile"),
        "check make/cflags inattendu sans Makefile : {checks:?}"
    );
    // La cible n'est pas salie (ni .o, ni .a, ni binaire).
    assert_eq!(
        root_entries(target.path()),
        vec!["lib.c", "main.c"],
        "cible salie par le build custom"
    );
}

#[test]
fn binary_build_command_en_echec_ko_build_failed() {
    let toml = r#"
[project]
name = "rush2"
type = "binary"
binary = "rush2"

[build]
command = "cc -o rush2 main.c"
"#;
    let (_bat_dir, battery) = load_battery(toml);
    let target = TempDir::new().unwrap();
    fs::write(
        target.path().join("main.c"),
        "int main(void) {\n    oops = ;\n}\n",
    )
    .unwrap();

    let (ok, events, ctx) = run_build(&battery, target.path());
    assert!(!ok, "commande de build en échec : KO attendu");
    let checks = extract_checks(&events);
    assert!(
        checks.iter().any(|(n, ok, d)| n == "build"
            && !*ok
            && d.contains("build failed: cc -o rush2 main.c")
            && d.contains("error")),
        "KO 'build failed: <command>' avec extrait absent : {checks:?}"
    );
    assert!(ctx.build.as_ref().and_then(|b| b.binary.as_ref()).is_none());
}

#[test]
fn pre_command_en_echec_ko_et_build_jamais_tente() {
    let toml = r#"
[project]
name = "rush2"
type = "binary"
binary = "rush2"

[build]
pre_commands = ["false"]
command = "touch rush2"
"#;
    let (_bat_dir, battery) = load_battery(toml);
    let target = TempDir::new().unwrap();
    fs::write(target.path().join("main.c"), RUSH2_MAIN).unwrap();

    let (ok, events, ctx) = run_build(&battery, target.path());
    assert!(!ok, "pre_command KO doit faire échouer l'étape");
    let checks = extract_checks(&events);
    assert!(
        checks.iter().any(|(n, ok, d)| n == "pre_command"
            && !*ok
            && d.contains("pre_command failed: false")),
        "KO 'pre_command failed' absent : {checks:?}"
    );
    assert!(
        checks.iter().all(|(n, _, _)| n != "build"),
        "la commande de build ne doit pas être tentée : {checks:?}"
    );
    // La commande n'a jamais tourné : pas de rush2 en salle blanche.
    let white = ctx
        .build
        .as_ref()
        .expect("ctx.build posé")
        .dir
        .path()
        .to_path_buf();
    assert!(
        !white.join("rush2").exists(),
        "build exécuté malgré la pre_command KO"
    );
}

#[test]
fn binary_build_command_avec_makefile_check_cflags_applique() {
    // Un Makefile EST présent (sans les cflags attendus) : le check
    // cflags s'applique et pénalise, sans bloquer ; jamais de make.
    let toml = r#"
[project]
name = "rush2"
type = "binary"
binary = "rush2"

[build]
command = "cc -o rush2 main.c"
"#;
    let (_bat_dir, battery) = load_battery(toml);
    let target = TempDir::new().unwrap();
    fs::write(
        target.path().join("main.c"),
        RUSH2_MAIN.replace(
            "int lib_fn(void);\n\n",
            "int lib_fn(void)\n{\n    return 42;\n}\n\n",
        ),
    )
    .unwrap();
    fs::write(target.path().join("Makefile"), "# Makefile sans cflags\n").unwrap();

    let (ok, events, _ctx) = run_build(&battery, target.path());
    assert!(ok, "cflags manquants : pénalité non bloquante : {events:?}");
    let checks = extract_checks(&events);
    assert!(
        checks
            .iter()
            .any(|(n, ok, d)| n == "cflags" && !*ok && d.contains("missing cflag: -Wall")),
        "pénalité cflags absente malgré le Makefile : {checks:?}"
    );
    assert!(
        checks.iter().all(|(n, _, _)| n != "make"),
        "make ne doit jamais tourner avec build.command : {checks:?}"
    );
}
