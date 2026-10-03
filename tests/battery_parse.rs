//! Tests du modèle de batterie TOML (Task 1).

use seeyou::battery::model::ProjectType;
use seeyou::battery::Battery;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tempfile::TempDir;

/// `XDG_CONFIG_HOME` est process-global : tous les tests qui y touchent
/// (directement ou via `discover`) se sérialisent sur ce mutex.
static XDG_MUTEX: Mutex<()> = Mutex::new(());

/// Positionne `XDG_CONFIG_HOME` et le restaure à sa valeur initiale au drop.
struct XdgConfigGuard(Option<std::ffi::OsString>);

impl XdgConfigGuard {
    fn set(path: &Path) -> Self {
        let ancien = std::env::var_os("XDG_CONFIG_HOME");
        std::env::set_var("XDG_CONFIG_HOME", path);
        Self(ancien)
    }
}

impl Drop for XdgConfigGuard {
    fn drop(&mut self) {
        match &self.0 {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }
}

/// Crée `<config>/.config/seeyou/batteries`, y écrit `files`
/// (nom → contenu) et renvoie le chemin du dossier `batteries`.
fn setup_config_batteries(config: &TempDir, files: &[(&str, &str)]) -> PathBuf {
    let batteries = config.path().join(".config/seeyou/batteries");
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
    // Config vide (pas de .config/seeyou/batteries) : env hermétique,
    // indépendante de la machine hôte.
    let config = TempDir::new().unwrap();
    let _xdg = XdgConfigGuard::set(&config.path().join(".config"));
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
    let _xdg = XdgConfigGuard::set(&config.path().join(".config"));
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
    let _xdg = XdgConfigGuard::set(&config.path().join(".config"));
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
    let _xdg = XdgConfigGuard::set(&config.path().join(".config"));
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
