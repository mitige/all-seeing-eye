# All-Seeing Eye

## Installation (une ligne, toute distro Linux x86_64)

```sh
mkdir -p ~/.local/bin && curl -sSL https://github.com/mitige/all-seeing-eye/releases/latest/download/all-seeing-eye -o ~/.local/bin/all-seeing-eye && chmod +x ~/.local/bin/all-seeing-eye
```

Binaire **statique** (musl) de la [dernière release](https://github.com/mitige/all-seeing-eye/releases/latest) — zéro dépendance, aucune compilation. Vérifie : `all-seeing-eye list`.

---

**Copie 1:1 de la moulinette Epitech** pour le programme **Bachelor**
(3 ans) et sa toute nouvelle **piscine C** (promo 2026-2027) — en
**local**, sur ta machine, avant le push.

La vraie moulinette corrige ton rendu après le push, dans son
environnement, sans pitié. All-Seeing Eye rejoue exactement ce
pipeline chez toi : mêmes étapes, même ordre, même sévérité, sortie
comparée au caractère près — avec en prime un **dashboard TUI live**
qui montre tout en temps réel et **toutes les erreurs de norme**
listées (via `epiclang`/`banana`, le checker officiel, quand il est
installé).

**16 batteries embarquées** couvrant la piscine C : Day03 à Day13,
Rush1, Rush2, Star, WorkshopLib et CountIsland — chacune validée à
100 % sur sa solution de référence (`cargo test --test batteries_all`).

Pipeline, dans l'ordre :

1. **Vérifications préliminaires** — fichiers interdits (C-O1),
   `banana-check-repo` s'il est installé, puis Makefile + règles
   (binary) ou rendus + prototypes (functions) ;
2. **Compilation** — en salle blanche (copie filtrée du rendu, jamais
   modifié) : `make fclean && make re` pour un binary, `cc -c` par
   delivery pour un functions ;
3. **Norme** — epiclang + banana si détecté, moteur interne de repli
   sinon ;
4. **Symboles** — fonctions interdites (binaire linké en projet `binary`, `.o` produits en `functions`)
   (whitelist `allowed_functions`) ;
5. **Tests unitaires** — `make tests_run`, sortie criterion parsée,
   si `tests_run_rule` ;
6. **Tests fonctionnels** — au caractère près : le binaire exécuté
   (binary), chaque task compilée avec son harness puis exécutée
   (functions) ;
7. **Verdict** — scores par groupe, score global, rapport JSON + texte.

Un build KO met les étapes qui en dépendent (unit, functional) en
skipped ; norme et symbols tournent quand même. Un groupe de tests
attendu mais vide compte **0 %** — jamais de 100 % vacuoleux. Le score
global est la moyenne simple des groupes.

## Installation

Prérequis (build depuis les sources uniquement) : Rust stable, `cc`/`gcc`, `make`. Optionnels : `epiclang`
(norme officielle 1:1), `banana-check-repo` (contrôles C-O en prelim),
`gcovr` (couverture relayée après `tests_run`), criterion (consommé
par les `make tests_run` des rendus).

```sh
cargo install --path .
```

Le binaire `all-seeing-eye` est installé dans `~/.cargo/bin`.

## Usage

```sh
cd mon_rendu
all-seeing-eye                        # TUI live, rapport texte au « q »
all-seeing-eye --no-tui               # flux texte (CI) — exit 0 si score 100 % ET toutes étapes OK, 1 sinon
all-seeing-eye --battery cpool_day03  # batterie nommée au lieu de la découverte
all-seeing-eye --strict-norme         # epiclang requis : norme KO s'il est absent
all-seeing-eye list                   # batteries connues (config + embarquées)
all-seeing-eye report                 # rouvre le dernier rapport (TUI ; texte avec --no-tui)
```

Découverte de la batterie, dans l'ordre : `./moulinette.toml` →
`~/.config/all-seeing-eye/batteries/*.toml` dont `project.name`
correspond au nom du dossier courant → batteries embarquées.

Batteries embarquées (C Pool 2026-2027, toutes validées à 100 % sur
leurs solutions de référence par `tests/batteries_all.rs`) :
`cpool_day03` (fonctions de base), `cpool_day04` (pointeurs),
`cpool_day05` (récursivité), `cpool_day06` (strings, 17 tasks),
`cpool_day07` (strcat + lib + programmes à args), `cpool_day08`
(allocation, word array, bases), `cpool_day09` (headers, structs,
couleurs), `cpool_day10` (do-op + advanced), `cpool_day11` (listes
chaînées, 11 tasks), `cpool_day12` (cat), `cpool_day13` (CSFML —
requiert la lib), `cpool_rush1` (5 carrés), `cpool_rush2` (détection
de langue), `cpool_star` (l'étoile), `cpool_workshoplib` (libmy, 30
fonctions), `cpool_countisland` (îles).

Rapports du dernier run : `~/.local/share/all-seeing-eye/last.json`
(machine) et `last.txt` (humain).

TUI : `↑`/`k` et `↓`/`j` naviguent dans l'arbre, `PgUp`/`PgDn`
scrollent le détail, `f` filtre les échecs, `e` exporte le rapport
texte, `q` (ou `Ctrl+C`) quitte.

## Format de batterie (TOML)

Une batterie = un TOML (`moulinette.toml` à la racine du rendu, ou
`<nom>.toml` dans le dossier batteries de la config) + ses assets
(harness, expected) en chemins relatifs au TOML. Tout chemin absolu ou
contenant `..` est rejeté.

### `[project]`

| champ | type | défaut | rôle |
|---|---|---|---|
| `name` | string | requis | nom du projet (= nom du dossier du rendu pour la découverte) |
| `type` | `"binary"` \| `"functions"` | requis | binary : un binaire à construire puis tester ; functions : des exercices piscine |
| `binary` | string | — | **requis si binary** : binaire attendu à la racine après `make re` |
| `makefile_rules` | string[] | `["all", "clean", "fclean", "re"]` | règles sondées par `make -n` en prelim (binary) |
| `cflags` | string[] | `["-Wall", "-Wextra", "-Werror"]` | flags de compilation ; les `-I*`/`-D*` sont aussi forwardés à la norme |
| `allowed_functions` | string[] | `[]` | whitelist de symboles autorisés (vide = étape Symboles désactivée) |
| `tests_run_rule` | bool | `false` | active l'étape Tests unitaires (`make tests_run`, criterion) |

### `[[task]]` (type functions) — un exercice

| champ | type | défaut | rôle |
|---|---|---|---|
| `name` | string | requis | nom du test dans le rapport |
| `delivery` | string | requis | fichier rendu, relatif au repo — ou **dossier** : tous ses `*.c` directs (triés, non récursif) sont livrés (Rush1 : `rush-1-1`, Day12 : `cat`) ; le prototype est vérifié dans leur concaténation |
| `prototype` | string | — | signature attendue, vérifiée en prelim (blancs normalisés) |
| `harness` | path | requis | `main` fourni par la batterie, relatif au TOML |
| `extra_sources` | path[] | `[]` | sources officiels liés au test (ex. `my_putchar.c`) |
| `args` | string[] | `[]` | arguments (argv[1..]) du harness compilé |
| `stdin` | string | `""` | écrit sur stdin du harness compilé |
| `include_dirs` | string[] | `[]` | deviennent `-I<dir>` à la compile, résolus en salle blanche |
| `link_flags` | string[] | `[]` | passés au link du harness (ex. `-Llib/my -lmy`), résolus en salle blanche |
| `stdout` / `stdout_file` | string / path | — | stdout attendu, inline ou fichier (mutuellement exclusifs) |
| `stderr` | string | `""` | stderr attendu |
| `exit_code` | int | `0` | exit code attendu (0..=255) |
| `timeout_ms` | int | `2000` | timeout de l'exécution |

Chaque task est compilée
`<cc> <cflags…> <sources de la delivery> <harness> <extra…> <-I…> <link_flags…> -o <stem>_test`
dans la salle blanche, puis exécutée avec ses `args` et son `stdin` ;
la sortie est comparée au caractère près (stdout, puis stderr, puis
exit code) — un écart produit un diff unified `-attendu` / `+obtenu`.

### `[[functional_test]]` (type binary) — un test end-to-end

| champ | type | défaut | rôle |
|---|---|---|---|
| `name` | string | requis | nom du test dans le rapport |
| `args` | string[] | `[]` | arguments (argv[1..]) |
| `stdin` | string | `""` | écrit sur stdin du binaire |
| `harness` | path | — | source C compilé et linké avec le binaire produit, relatif au TOML (WorkshopLib : le produit est une `lib/libmy.a`) — `<cc> <cflags…> <harness> <link_flags…> <produit> -o harness_test_<n>`, et c'est CE binaire qui est exécuté |
| `link_flags` | string[] | `[]` | flags de link du harness, résolus en salle blanche |
| `stdout` / `stdout_file` | string / path | — | stdout attendu, inline ou fichier (mutuellement exclusifs) |
| `stderr` | string | `""` | stderr attendu |
| `exit_code` | int | `0` | exit code attendu (0..=255) |
| `timeout_ms` | int | `2000` | timeout de l'exécution |

Sans `harness`, le binaire produit doit être directement exécutable :
un produit non exécutable (ex. une lib statique) testé sans harness
est un KO clair (« binaire non exécutable sans harness »).

### `[build]` (optionnel, top-level) — build sur mesure

| champ | type | défaut | rôle |
|---|---|---|---|
| `pre_commands` | string[] | `[]` | commandes shell (`/bin/sh -c`) jouées en salle blanche AVANT le build, binary ou functions (ex. `cd lib/my && ./build.sh`) ; le premier échec fait échouer l'étape (`pre_command failed: <cmd>`) |
| `command` | string | — | si présent : cette commande shell REMPLACE `make fclean` + `make re` (binary, ex. Rush2) ; échec → `build failed: <command>` |

Avec `command` : la présence du binaire annoncé reste vérifiée après
la commande ; les sondes de règles Makefile du prelim sont skippées
(make n'est plus le mécanisme de build) ; le check des cflags n'est
appliqué que si un Makefile existe.

### `[[prototype]]`

Déclaratif, réservé : parsé (`name`, `signature`, `file`) mais non
consommé par le pipeline en V1 — le check de prototype utilise
`task.prototype`.

### Exemple — extrait de la batterie embarquée `cpool_day03`

```toml
[project]
name = "cpool_day03"
type = "functions"
allowed_functions = ["my_putchar"]
cflags = ["-Wall", "-Wextra"]

[[task]]
name = "my_print_alpha"
delivery = "my_print_alpha.c"
prototype = "int my_print_alpha(void)"
harness = "cpool_day03/harness/main_my_print_alpha.c"
extra_sources = ["cpool_day03/harness/my_putchar.c"]
stdout_file = "cpool_day03/expected/my_print_alpha.out"
```

## Calibrage de la norme

- **epiclang détecté** (`epiclang --version` lançable) : norme
  **officielle** — `epiclang -fsyntax-only` + plugin banana, par
  fichier, cwd = rendu. C'est le seul chemin strictement 1:1 avec la
  vraie moulinette. `--strict-norme` en fait une exigence : sans
  epiclang, l'étape Norme est KO au lieu de dégrader.
- **sinon, fallback interne** : moteur Rust calibré sur les messages
  exacts de banana, couvrant les règles mécaniques du coding style
  v7.1 (C-G1, C-G4, C-G6, C-G7, C-H2, C-L1, C-L4, C-L6, C-F2, C-F3,
  C-F4, C-F5, C-F8, C-A3, C-C3, C-O3), Makefile inclus. Ce n'est
  **pas** un parseur C : installez epiclang pour le 1:1 strict.

Côté compilation, la détection suit epiclang → cc → gcc.

## Structure du repo

```
src/
  cli.rs        CLI (run/list/report, détection compilateur)
  main.rs       entrée, code de sortie
  exec.rs       run_capture : spawn borné, rlimits, kill du groupe
  report.rs     scores, rendu texte, last.json / last.txt
  battery/      modèle TOML, découverte, batteries embarquées
  engine/       les 7 étapes (prelim, build, norme, symbols, unit,
                functional, verdict) + events
  norme/        official.rs (epiclang/banana), internal.rs (fallback)
  tui/          dashboard ratatui (arbre, détail, touches)
batteries/      batteries embarquées (cpool_day03 + harness/expected)
tests/          intégration par étape, E2E pipeline, fixtures
```

## Limites connues

- **Logs par batch post-mortem** : les sorties des sous-processus
  (make, compiles) sont capturées puis relayées en `LogLine` à la fin
  de chaque commande — pas de streaming live pendant un long build.
- **cwd des tests fonctionnels = la salle blanche** : pas de cwd
  dédié par test en V1 (le jour où la batterie portera un champ `cwd`,
  il sera relatif à la salle blanche).
- **Norme interne ⊂ banana** : le fallback ne couvre qu'un
  sous-ensemble mécanique des règles ; pour le 1:1, epiclang est
  requis (`--strict-norme` le garantit).
- **Crash criterion non-verbose non nommé** : un crash visible
  seulement au résumé (`Tests: X | Passing: Y | Failing: Z`), sans
  ligne `[FAIL]` qui le nomme, est matérialisé « criterion #N » en
  **Failed**, pas Crashed — criterion ne donne ni nom ni signal à ce
  niveau.
- **Linux only** : rlimits, `setsid`, kill de groupe de processus. Un
  descendant qui appelle lui-même `setsid` échappe au kill au timeout
  (fuite bornée à sa durée de vie, jamais de blocage).
- **Pas de `r` (relancer) dans le TUI en V1** : relance la commande.
- **Exit code 0 possible avec des fautes de norme** : le score global ne
  pondère que les groupes de tests ; la norme est rapportée, pas bloquante
  (comme la vraie moulinette qui note le style à part). Pour gater une CI
  sur la norme, lis `last.json` (`norme` + `scores.norme_*`).
