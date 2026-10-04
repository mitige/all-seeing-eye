//! Tests du modèle de batterie TOML (Task 1).

use all_seeing_eye::battery::model::ProjectType;
use all_seeing_eye::battery::Battery;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

mod common;

use common::{XdgGuard, XDG_MUTEX};

/// Positionne `XDG_CONFIG_HOME` (process-global — les tests qui y
/// touchent, directement ou via `discover`, se sérialisent sur
/// [`XDG_MUTEX`]) et le restaure à sa valeur initiale au drop.
fn xdg_config(path: &std::path::Path) -> XdgGuard {
    XdgGuard::set("XDG_CONFIG_HOME", path)
}

/// Crée `<config>/.config/all-seeing-eye/batteries`, y écrit `files`
/// (nom → contenu) et renvoie le chemin du dossier `batteries`.
fn setup_config_batteries(config: &TempDir, files: &[(&str, &str)]) -> PathBuf {
    let batteries = config.path().join(".config/all-seeing-eye/batteries");
    fs::create_dir_all(&batteries).unwrap();
    for (nom, contenu) in files {
        fs::write(batteries.join(nom), contenu).unwrap();
    }
    batteries
}

/// Écrit un `moulinette.toml` dans `dir` et renvoie son chemin.
fn write_battery(dir: &TempDir, content: &str) -> PathBuf {
    let path = dir.path().join("moulinette.toml");
    fs::write(&path, content).unwrap();
    path
}

/// Batterie « functions » complète, inspirée du C Pool day03.
const DAY03_TOML: &str = r#"
[project]
name = "cpool_day03"
type = "functions"
allowed_functions = ["write"]

[[prototype]]
name = "my_print_alpha"
signature = "void my_print_alpha(void);"
file = "my_print_alpha.c"

[[task]]
name = "ex01"
delivery = "my_print_alpha.c"
prototype = "void my_print_alpha(void);"
harness = "harness/ex01_main.c"
extra_sources = ["lib/my_putchar.c"]
stdout = "abcdefghijklmnopqrstuvwxyz"

[[task]]
name = "ex02"
delivery = "my_print_revalpha.c"
harness = "harness/ex02_main.c"
stdout_file = "expected/ex02.txt"
timeout_ms = 5000
"#;

#[test]
fn parse_batterie_day03_complete() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("expected")).unwrap();
    fs::write(
        dir.path().join("expected/ex02.txt"),
        "zyxwvutsrqponmlkjihgfedcba\n",
    )
    .unwrap();
    let path = write_battery(&dir, DAY03_TOML);

    let b = Battery::load(&path).unwrap();

    // [project]
    assert_eq!(b.project.name, "cpool_day03");
    assert_eq!(b.project.kind, ProjectType::Functions);
    assert_eq!(b.project.binary, None);
    assert_eq!(b.project.allowed_functions, vec!["write".to_string()]);
    assert!(!b.project.tests_run_rule);

    // [[prototype]]
    assert_eq!(b.prototype.len(), 1);
    assert_eq!(b.prototype[0].name, "my_print_alpha");
    assert_eq!(b.prototype[0].signature, "void my_print_alpha(void);");
    assert_eq!(b.prototype[0].file, "my_print_alpha.c");

    // root = parent du TOML
    assert_eq!(b.root, dir.path());

    // [[task]] ex01 : stdout inline, defaults appliqués
    assert_eq!(b.task.len(), 2);
    let ex01 = &b.task[0];
    assert_eq!(ex01.name, "ex01");
    assert_eq!(ex01.delivery, "my_print_alpha.c");
    assert_eq!(
        ex01.prototype.as_deref(),
        Some("void my_print_alpha(void);")
    );
    assert_eq!(ex01.harness, PathBuf::from("harness/ex01_main.c"));
    assert_eq!(ex01.extra_sources, vec![PathBuf::from("lib/my_putchar.c")]);
    assert_eq!(ex01.stderr, "");
    assert_eq!(ex01.exit_code, 0);
    assert_eq!(ex01.timeout_ms, 2000);
    assert_eq!(
        b.expected_stdout_of_task(ex01).unwrap(),
        "abcdefghijklmnopqrstuvwxyz"
    );

    // [[task]] ex02 : stdout via fichier relatif à root, timeout surchargé
    let ex02 = &b.task[1];
    assert_eq!(ex02.timeout_ms, 5000);
    assert_eq!(ex02.stdout, None);
    assert_eq!(
        b.expected_stdout_of_task(ex02).unwrap(),
        "zyxwvutsrqponmlkjihgfedcba\n"
    );
}

#[test]
fn defauts_appliques_projet_binaire() {
    let toml = r#"
[project]
name = "mysh"
type = "binary"
binary = "mysh"

[[functional_test]]
name = "affiche hello"
args = ["-c", "echo hello"]
stdout = "hello\n"
"#;
    let dir = TempDir::new().unwrap();
    let path = write_battery(&dir, toml);

    let b = Battery::load(&path).unwrap();

    // Defaults [project]
    assert_eq!(
        b.project.makefile_rules,
        ["all", "clean", "fclean", "re"]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        b.project.cflags,
        ["-Wall", "-Wextra", "-Werror"]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    );
    assert!(b.project.allowed_functions.is_empty());
    assert!(!b.project.tests_run_rule);
    assert_eq!(b.project.binary.as_deref(), Some("mysh"));

    // Defaults [[functional_test]]
    let t = &b.functional_test[0];
    assert_eq!(t.name, "affiche hello");
    assert_eq!(t.args, vec!["-c", "echo hello"]);
    assert_eq!(t.stdin, "");
    assert_eq!(t.stderr, "");
    assert_eq!(t.exit_code, 0);
    assert_eq!(t.timeout_ms, 2000);
    assert_eq!(b.expected_stdout_of_test(t).unwrap(), "hello\n");
}

#[test]
fn stdout_et_stdout_file_exclusifs_sur_task() {
    let toml = r#"
[project]
name = "cpool_day03"
type = "functions"

[[task]]
name = "ex01"
delivery = "my_print_alpha.c"
harness = "harness/ex01_main.c"
stdout = "abc"
stdout_file = "expected/ex01.txt"
"#;
    let dir = TempDir::new().unwrap();
    let path = write_battery(&dir, toml);

    let err = Battery::load(&path).unwrap_err();
    assert!(
        err.to_string().contains("mutuellement exclusifs"),
        "message inattendu : {err}"
    );
}

#[test]
fn stdout_et_stdout_file_exclusifs_sur_functional_test() {
    let toml = r#"
[project]
name = "mysh"
type = "binary"
binary = "mysh"

[[functional_test]]
name = "hello"
stdout = "hello\n"
stdout_file = "expected/hello.txt"
"#;
    let dir = TempDir::new().unwrap();
    let path = write_battery(&dir, toml);

    let err = Battery::load(&path).unwrap_err();
    assert!(
        err.to_string().contains("mutuellement exclusifs"),
        "message inattendu : {err}"
    );
}

#[test]
fn binary_sans_champ_binary_erreur() {
    let toml = r#"
[project]
name = "mysh"
type = "binary"
"#;
    let dir = TempDir::new().unwrap();
    let path = write_battery(&dir, toml);

    let err = Battery::load(&path).unwrap_err();
    assert!(
        err.to_string().contains("binary"),
        "message inattendu : {err}"
    );
}

#[test]
fn functions_sans_task_erreur() {
    let toml = r#"
[project]
name = "cpool_day03"
type = "functions"
"#;
    let dir = TempDir::new().unwrap();
    let path = write_battery(&dir, toml);

    let err = Battery::load(&path).unwrap_err();
    assert!(
        err.to_string().contains("task"),
        "message inattendu : {err}"
    );
}

#[test]
fn expected_stdout_erreur_si_aucune_source() {
    let toml = r#"
[project]
name = "cpool_day03"
type = "functions"

[[task]]
name = "ex01"
delivery = "my_print_alpha.c"
harness = "harness/ex01_main.c"
"#;
    let dir = TempDir::new().unwrap();
    let path = write_battery(&dir, toml);

    let b = Battery::load(&path).unwrap();
    let err = b.expected_stdout_of_task(&b.task[0]).unwrap_err();
    assert!(
        err.to_string().contains("ex01"),
        "message inattendu : {err}"
    );
}

#[test]
fn champ_inconnu_dans_functional_test_erreur() {
    let toml = r#"
[project]
name = "mysh"
type = "binary"
binary = "mysh"

[[functional_test]]
name = "hello"
stdout = "hello\n"
timout_ms = 9999
"#;
    let dir = TempDir::new().unwrap();
    let path = write_battery(&dir, toml);

    let err = Battery::load(&path).unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("timout_ms"),
        "la coquille doit être signalée, message : {msg}"
    );
}

/// Charge un TOML et renvoie l'erreur complète (panique si accepté).
fn load_err(toml: &str) -> String {
    let dir = TempDir::new().unwrap();
    let path = write_battery(&dir, toml);
    format!("{:#}", Battery::load(&path).unwrap_err())
}

/// Batterie Binary avec un champ `binary` paramétré.
fn toml_binary(binary: &str) -> String {
    format!("[project]\nname = \"x\"\ntype = \"binary\"\nbinary = \"{binary}\"\n")
}

/// Batterie Functions : une task, avec un `champ` TOML additionnel
/// injecté (ligne(s) brute(s) après stdout).
fn toml_functions(champ: &str) -> String {
    format!(
        "[project]\nname = \"x\"\ntype = \"functions\"\n\n\
         [[task]]\nname = \"ex01\"\ndelivery = \"a.c\"\n\
         harness = \"harness/ex01_main.c\"\nstdout = \"\"\n{champ}"
    )
}

#[test]
fn validate_rejette_binary_absolu_ou_parent_dir() {
    for hostile in ["/bin/sh", "../sh", "a/../../sh"] {
        let err = load_err(&toml_binary(hostile));
        assert!(
            err.contains("binary") && err.contains(hostile),
            "binary hostile « {hostile} » non rejeté proprement : {err}"
        );
    }
}

#[test]
fn validate_rejette_delivery_absolue_ou_parent_dir() {
    for hostile in ["/etc/passwd.c", "../a.c"] {
        let toml = toml_functions("").replacen("a.c", hostile, 1);
        let err = load_err(&toml);
        assert!(
            err.contains("delivery") && err.contains(hostile),
            "delivery hostile « {hostile} » non rejetée proprement : {err}"
        );
    }
}

#[test]
fn validate_rejette_harness_stdout_file_extra_sources_hostiles() {
    // (champ attendu dans le message, valeur hostile, ligne TOML)
    let cas: &[(&str, &str, &str)] = &[
        ("harness", "../h.c", ""), // harness substitué plus bas
        ("harness", "/abs/h.c", ""),
        (
            "stdout_file",
            "/etc/passwd",
            "stdout_file = \"/etc/passwd\"\n",
        ),
        ("stdout_file", "../ex.txt", "stdout_file = \"../ex.txt\"\n"),
        (
            "extra_sources",
            "../lib.c",
            "extra_sources = [\"../lib.c\"]\n",
        ),
    ];
    for (champ, hostile, ligne) in cas {
        let mut toml = toml_functions(ligne);
        if *champ == "harness" {
            toml = toml.replacen("harness/ex01_main.c", hostile, 1);
        }
        let err = load_err(&toml);
        assert!(
            err.contains(champ) && err.contains(hostile),
            "{champ} hostile « {hostile} » non rejeté proprement : {err}"
        );
    }
}

#[test]
fn validate_rejette_stdout_file_hostile_sur_functional_test() {
    let toml = "[project]\nname = \"x\"\ntype = \"binary\"\nbinary = \"x\"\n\n\
                [[functional_test]]\nname = \"t\"\nstdout_file = \"../x.txt\"\n";
    let err = load_err(toml);
    assert!(
        err.contains("stdout_file") && err.contains("../x.txt"),
        "stdout_file hostile non rejeté sur functional_test : {err}"
    );
}

/// Batterie Binary : un functional_test, avec un `champ` TOML
/// additionnel injecté (ligne(s) brute(s) après stdout).
fn toml_functional_test(champ: &str) -> String {
    format!(
        "[project]\nname = \"x\"\ntype = \"binary\"\nbinary = \"x\"\n\n\
         [[functional_test]]\nname = \"t\"\nstdout = \"\"\n{champ}"
    )
}

#[test]
fn validate_rejette_exit_code_hors_0_255() {
    // Sur Task comme sur FunctionalTest : un exit code hors plage
    // POSIX (0..=255) est une erreur de batterie.
    for mauvais in ["256", "-1"] {
        let err = load_err(&toml_functions(&format!("exit_code = {mauvais}\n")));
        assert!(
            err.contains("exit_code") && err.contains(mauvais),
            "exit_code = {mauvais} (task) non rejeté proprement : {err}"
        );
        let err = load_err(&toml_functional_test(&format!("exit_code = {mauvais}\n")));
        assert!(
            err.contains("exit_code") && err.contains(mauvais),
            "exit_code = {mauvais} (functional_test) non rejeté proprement : {err}"
        );
    }
}

#[test]
fn validate_rejette_timeout_ms_zero() {
    // Un timeout nul rendrait tout test KO instantané : refusé sur
    // Task comme sur FunctionalTest.
    let err = load_err(&toml_functions("timeout_ms = 0\n"));
    assert!(
        err.contains("timeout_ms"),
        "timeout_ms = 0 (task) non rejeté proprement : {err}"
    );
    let err = load_err(&toml_functional_test("timeout_ms = 0\n"));
    assert!(
        err.contains("timeout_ms"),
        "timeout_ms = 0 (functional_test) non rejeté proprement : {err}"
    );
}

#[test]
fn validate_accepte_les_chemins_relatifs_propres() {
    // Sous-dossiers relatifs légitimes : harness, extra_sources — aucun
    // rejet. (stdout_file est testé à part : exclusif avec stdout.)
    let toml = toml_functions("extra_sources = [\"lib/my_putchar.c\", \"src/deep/util.c\"]\n");
    let dir = TempDir::new().unwrap();
    let path = write_battery(&dir, &toml);
    assert!(
        Battery::load(&path).is_ok(),
        "chemins relatifs propres rejetés : {:?}",
        Battery::load(&path).err()
    );

    let toml =
        toml_functions("stdout_file = \"expected/ex01.txt\"\n").replacen("stdout = \"\"\n", "", 1);
    let dir = TempDir::new().unwrap();
    let path = write_battery(&dir, &toml);
    assert!(
        Battery::load(&path).is_ok(),
        "stdout_file relatif rejeté : {:?}",
        Battery::load(&path).err()
    );
}

#[test]
fn discover_trouve_le_moulinette_toml_local() {
    let dir = TempDir::new().unwrap();
    write_battery(&dir, DAY03_TOML);

    let b = Battery::discover(dir.path()).unwrap();
    assert_eq!(b.project.name, "cpool_day03");
    assert_eq!(b.root, dir.path());
}

#[test]
fn discover_erreur_si_aucune_batterie() {
    let _lock = XDG_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    // Config vide (pas de .config/all-seeing-eye/batteries) : env hermétique,
    // indépendante de la machine hôte.
    let config = TempDir::new().unwrap();
    let _xdg = xdg_config(&config.path().join(".config"));
    let dir = TempDir::new().unwrap(); // dossier vide, nom aléatoire

    let err = Battery::discover(dir.path()).unwrap_err();
    assert!(
        err.to_string().contains("aucune batterie trouvée"),
        "message inattendu : {err}"
    );
}

/// Batterie binaire minimale valide, `project.name = "mysh"`.
const MYSH_TOML: &str = r#"
[project]
name = "mysh"
type = "binary"
binary = "mysh"

[[functional_test]]
name = "hello"
stdout = "hello\n"
"#;

/// Invalide : projet binary sans champ `binary`.
const CASSEE_TOML: &str = r#"
[project]
name = "mysh"
type = "binary"
"#;

#[test]
fn discover_tier2_trouve_batterie_par_nom_de_dossier() {
    let _lock = XDG_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    let config = TempDir::new().unwrap();
    let _xdg = xdg_config(&config.path().join(".config"));
    let batteries = setup_config_batteries(&config, &[("mysh.toml", MYSH_TOML)]);
    let work = TempDir::new().unwrap();
    let projet = work.path().join("mysh");
    fs::create_dir(&projet).unwrap();

    let b = Battery::discover(&projet).unwrap();

    assert_eq!(b.project.name, "mysh");
    assert_eq!(b.root, batteries);
}

#[test]
fn discover_tier2_nom_non_matchant_erreur() {
    let _lock = XDG_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    let config = TempDir::new().unwrap();
    let _xdg = xdg_config(&config.path().join(".config"));
    setup_config_batteries(&config, &[("mysh.toml", MYSH_TOML)]);
    let work = TempDir::new().unwrap();
    let projet = work.path().join("autre_projet");
    fs::create_dir(&projet).unwrap();

    let err = Battery::discover(&projet).unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("aucune batterie trouvée"),
        "message inattendu : {msg}"
    );
    assert!(
        !msg.contains("mysh.toml"),
        "une batterie valide ne doit pas être listée comme invalide : {msg}"
    );
}

#[test]
fn discover_tier2_batterie_invalide_mentionnee() {
    let _lock = XDG_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    let config = TempDir::new().unwrap();
    let _xdg = xdg_config(&config.path().join(".config"));
    setup_config_batteries(&config, &[("cassee.toml", CASSEE_TOML)]);
    let work = TempDir::new().unwrap();
    let projet = work.path().join("projet_inconnu");
    fs::create_dir(&projet).unwrap();

    let err = Battery::discover(&projet).unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("aucune batterie trouvée"),
        "message inattendu : {msg}"
    );
    assert!(
        msg.contains("cassee.toml"),
        "le fichier cassé doit être mentionné : {msg}"
    );
    assert!(
        msg.contains("binary"),
        "l'erreur de la batterie doit être mentionnée : {msg}"
    );
}
