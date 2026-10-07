//! Tests du modèle de batterie TOML (Task 1).

use all_seeing_eye::battery::model::ProjectType;
use all_seeing_eye::battery::{self, Battery};
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

// ── Task 13 : batterie embarquée cpool_day03 ─────────────────────

/// Les 8 tasks du Day03, dans l'ordre du sujet.
const DAY03_TASKS: [&str; 8] = [
    "my_print_alpha",
    "my_print_revalpha",
    "my_print_digits",
    "my_isneg",
    "my_print_comb",
    "my_print_comb2",
    "my_put_nbr",
    "my_print_combn",
];

#[test]
fn embedded_batteries_liste_cpool_day03() {
    let (_, contenu) = battery::embedded_batteries()
        .into_iter()
        .find(|(n, _)| *n == "cpool_day03")
        .expect("cpool_day03 doit être embarquée");
    let b: Battery = toml::from_str(contenu).expect("TOML embarqué invalide");
    assert_eq!(b.project.name, "cpool_day03");
    assert_eq!(b.project.kind, ProjectType::Functions);
    let noms: Vec<&str> = b.task.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(noms, DAY03_TASKS);
}

#[test]
fn extract_embedded_inconnue_est_none_et_nom_valide() {
    assert!(battery::extract_embedded("nope").unwrap().is_none());
    // Un « nom » qui est un chemin est rejeté : jamais de résolution
    // hors du contenu embarqué.
    for hostile in ["", ".", "..", "a/b", "a\\b"] {
        assert!(
            battery::extract_embedded(hostile).is_err(),
            "accepté : « {hostile} »"
        );
    }
}

#[test]
fn extract_embedded_recopie_toml_harness_et_expected() {
    let tmp = battery::extract_embedded("cpool_day03")
        .unwrap()
        .expect("cpool_day03 embarquée");
    let root = tmp.path();
    let toml = root.join("cpool_day03.toml");
    assert!(toml.is_file(), "TOML extrait manquant");
    let harness = root.join("cpool_day03/harness");
    assert!(harness.join("my_putchar.c").is_file());
    for t in DAY03_TASKS {
        assert!(
            harness.join(format!("main_{t}.c")).is_file(),
            "harness main_{t}.c manquant"
        );
        assert!(
            root.join(format!("cpool_day03/expected/{t}.out")).is_file(),
            "expected {t}.out manquant"
        );
    }
    // Le TOML extrait se charge : root = tempdir, stdout_file résolus.
    let b = Battery::load(&toml).unwrap();
    assert_eq!(b.root, root);
    for t in &b.task {
        b.expected_stdout_of_task(t)
            .unwrap_or_else(|e| panic!("stdout de « {} » illisible : {e:#}", t.name));
    }
    // Spot-checks contre le sujet (sorties brutes, sans newline final).
    let alpha = b.expected_stdout_of_task(&b.task[0]).unwrap();
    assert_eq!(alpha, "abcdefghijklmnopqrstuvwxyz");
    let comb = &b.expected_stdout_of_task(&b.task[4]).unwrap();
    assert!(comb.starts_with("012, 013, 014") && comb.ends_with(", 789"));
    assert!(!comb.contains("987") && !comb.contains("999"));
    let nbr = b.expected_stdout_of_task(&b.task[6]).unwrap();
    assert!(nbr.contains("-2147483648"), "INT_MIN absent : {nbr}");
}

#[test]
fn load_embedded_inconnue_est_none() {
    assert!(battery::load_embedded("nope").unwrap().is_none());
}

#[test]
fn discover_tier3_embarquee_par_nom_de_dossier() {
    let _lock = XDG_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    // Config vide : les tiers 1 et 2 ne matchent pas, seul le tier 3
    // (embarquées) peut répondre.
    let config = TempDir::new().unwrap();
    let _xdg = xdg_config(&config.path().join(".config"));
    let work = TempDir::new().unwrap();
    let projet = work.path().join("cpool_day03");
    fs::create_dir(&projet).unwrap();

    let b = Battery::discover(&projet).unwrap();

    assert_eq!(b.project.name, "cpool_day03");
    // root pointe dans le TempDir d'extraction, qui doit être VIVANT
    // (détenu par la batterie) : les assets restent lisibles après le
    // retour de discover — y compris depuis un clone après drop de
    // l'original (le pipeline clone la batterie dans son thread).
    let b2 = b.clone();
    drop(b);
    assert!(
        b2.root.join("cpool_day03/harness/my_putchar.c").is_file(),
        "le TempDir d'extraction n'a pas survécu (root : {})",
        b2.root.display()
    );
    assert!(b2.expected_stdout_of_task(&b2.task[0]).is_ok());
}

// ── Extensions : dossiers, args/stdin, [build], harness + link_flags ──

#[test]
fn task_accepte_args_stdin_include_dirs_et_link_flags() {
    // Façon CountIsland : le harness compilé est exécuté avec des args
    // et un stdin, et la compile reçoit des -I et des flags de link.
    let toml = r#"
[project]
name = "count_island"
type = "functions"

[[task]]
name = "count_island"
delivery = "count_island.c"
harness = "harness/main.c"
args = ["3", "map.txt"]
stdin = "ligne\n"
include_dirs = ["include", "lib/my"]
link_flags = ["-Llib/my", "-lmy"]
stdout = ""
"#;
    let dir = TempDir::new().unwrap();
    let path = write_battery(&dir, toml);

    let b = Battery::load(&path).unwrap();
    let t = &b.task[0];
    assert_eq!(t.args, vec!["3".to_string(), "map.txt".to_string()]);
    assert_eq!(t.stdin, "ligne\n");
    assert_eq!(
        t.include_dirs,
        vec!["include".to_string(), "lib/my".to_string()]
    );
    assert_eq!(
        t.link_flags,
        vec!["-Llib/my".to_string(), "-lmy".to_string()]
    );
}

#[test]
fn task_et_batterie_nouveaux_champs_defauts_vides() {
    // Aucun des nouveaux champs dans le TOML : defaults appliqués.
    let dir = TempDir::new().unwrap();
    let path = write_battery(&dir, &toml_functions(""));

    let b = Battery::load(&path).unwrap();
    let t = &b.task[0];
    assert!(t.args.is_empty(), "args par défaut : {:?}", t.args);
    assert_eq!(t.stdin, "");
    assert!(t.include_dirs.is_empty());
    assert!(t.link_flags.is_empty());
    assert!(b.build.is_none(), "pas de [build] par défaut");
}

#[test]
fn functional_test_accepte_harness_et_link_flags() {
    // Façon WorkshopLib : le binaire produit est une .a, le test est
    // un harness compilé et linké avec elle.
    let toml = r#"
[project]
name = "workshop_lib"
type = "binary"
binary = "lib/libmy.a"

[[functional_test]]
name = "my_strlen"
harness = "harness/my_strlen.c"
link_flags = ["-Llib/my", "-lmy"]
stdout = "5\n"
"#;
    let dir = TempDir::new().unwrap();
    let path = write_battery(&dir, toml);

    let b = Battery::load(&path).unwrap();
    let t = &b.functional_test[0];
    assert_eq!(t.harness, Some(PathBuf::from("harness/my_strlen.c")));
    assert_eq!(
        t.link_flags,
        vec!["-Llib/my".to_string(), "-lmy".to_string()]
    );
}

#[test]
fn functional_test_harness_et_link_flags_par_defaut() {
    let dir = TempDir::new().unwrap();
    let path = write_battery(&dir, &toml_functional_test(""));

    let b = Battery::load(&path).unwrap();
    let t = &b.functional_test[0];
    assert_eq!(t.harness, None);
    assert!(t.link_flags.is_empty());
}

#[test]
fn build_spec_parse_pre_commands_et_command() {
    // Façon Rush2 : la lib est construite par un script, puis le
    // binaire par une commande qui remplace make fclean/re.
    let toml = r#"
[project]
name = "rush2"
type = "binary"
binary = "rush2"

[build]
pre_commands = ["cd lib/my && ./build.sh"]
command = "clang -o rush2 *.c -I./include -L./lib/my -lmy"
"#;
    let dir = TempDir::new().unwrap();
    let path = write_battery(&dir, toml);

    let b = Battery::load(&path).unwrap();
    let spec = b.build.as_ref().expect("[build] doit être parsé");
    assert_eq!(
        spec.pre_commands,
        vec!["cd lib/my && ./build.sh".to_string()]
    );
    assert_eq!(
        spec.command.as_deref(),
        Some("clang -o rush2 *.c -I./include -L./lib/my -lmy")
    );
}

#[test]
fn build_spec_table_vide_defauts() {
    let toml = "[project]\nname = \"x\"\ntype = \"binary\"\nbinary = \"x\"\n\n[build]\n";
    let dir = TempDir::new().unwrap();
    let path = write_battery(&dir, toml);

    let b = Battery::load(&path).unwrap();
    let spec = b.build.as_ref().expect("[build] vide doit être parsé");
    assert!(spec.pre_commands.is_empty());
    assert!(spec.command.is_none());
}

#[test]
fn build_spec_champ_inconnu_erreur() {
    // deny_unknown_fields sur [build] : une coquille est signalée.
    let toml = "[project]\nname = \"x\"\ntype = \"binary\"\nbinary = \"x\"\n\n\
                [build]\ncomand = \"make\"\n";
    let err = load_err(toml);
    assert!(
        err.contains("comand"),
        "la coquille doit être signalée, message : {err}"
    );
}

#[test]
fn validate_rejette_harness_hostile_sur_functional_test() {
    // Le harness d'un functional_test est relu depuis battery.root :
    // mêmes garde-fous de chemin que le harness d'une task.
    for hostile in ["../h.c", "/abs/h.c"] {
        let toml = toml_functional_test(&format!("harness = \"{hostile}\"\n"));
        let err = load_err(&toml);
        assert!(
            err.contains("harness") && err.contains(hostile),
            "harness hostile « {hostile} » (functional_test) non rejeté proprement : {err}"
        );
    }
}

#[test]
fn validate_rejette_include_dirs_hostiles() {
    // Les include_dirs d'une task deviennent des -I résolus en salle
    // blanche : un chemin sortant du rendu est rejeté comme les
    // autres champs de chemin.
    for hostile in ["../include", "/abs/include"] {
        let toml = toml_functions(&format!("include_dirs = [\"{hostile}\"]\n"));
        let err = load_err(&toml);
        assert!(
            err.contains("include_dirs") && err.contains(hostile),
            "include_dirs hostile « {hostile} » non rejeté proprement : {err}"
        );
    }
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
