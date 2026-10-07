/*
** EPITECH PROJECT, 2026
** cpool_countisland
** File description:
** main de test pour count_island : mondes en dur, retour + grille
*/

/*
** Le harness declare lui-meme les prototypes (la vraie moulinette
** ajoute SON main.c) : la compile ne depend pas du my.h de
** l'etudiant. L'affichage passe par la libmy de l'etudiant
** (link_flags -Llib/my -lmy) : la lib est prouvee par l'usage, un
** symbole manquant fait un KO de link. Les lignes des mondes sont
** des tableaux modifiables (count_island reecrit les X) — jamais de
** litteraux de chaine, qui sont en memoire read-only.
*/

int count_island(char **world);
void my_putchar(char c);
int my_putstr(char const *str);
int my_put_nbr(int nb);

static void print_world(char **world)
{
    int i = 0;

    while (world[i] != 0) {
        my_putstr(world[i]);
        my_putchar('\n');
        i = i + 1;
    }
}

static void run_world(char const *name, char **world)
{
    int ret = count_island(world);

    my_putstr("== ");
    my_putstr(name);
    my_putstr(" ==\n");
    my_put_nbr(ret);
    my_putchar('\n');
    print_world(world);
}

static void case_empty_world(void)
{
    char *world[] = {0};

    run_world("empty_world", world);
}

static void case_blank_world(void)
{
    char l0[] = "....";
    char l1[] = "....";
    char l2[] = "....";
    char *world[] = {l0, l1, l2, 0};

    run_world("blank_world", world);
}

static void case_one_island(void)
{
    char l0[] = "XX..";
    char l1[] = "XX..";
    char l2[] = "....";
    char l3[] = "....";
    char *world[] = {l0, l1, l2, l3, 0};

    run_world("one_island", world);
}

static void case_two_islands(void)
{
    char l0[] = ".X..";
    char l1[] = "....";
    char l2[] = "..XX";
    char l3[] = "....";
    char *world[] = {l0, l1, l2, l3, 0};

    run_world("two_islands", world);
}

static void case_diagonal_trap(void)
{
    char l0[] = "X.X";
    char l1[] = ".X.";
    char l2[] = "X.X";
    char *world[] = {l0, l1, l2, 0};

    run_world("diagonal_trap", world);
}

static void case_l_shape(void)
{
    char l0[] = "XX.";
    char l1[] = "X..";
    char l2[] = "..X";
    char *world[] = {l0, l1, l2, 0};

    run_world("l_shape", world);
}

static void case_single_x(void)
{
    char l0[] = "X";
    char *world[] = {l0, 0};

    run_world("single_x", world);
}

static void case_big_map(void)
{
    char l0[] = "..........";
    char l1[] = ".XX....XX.";
    char l2[] = ".XX....XX.";
    char l3[] = "..........";
    char l4[] = "....X.....";
    char l5[] = "....X..X..";
    char l6[] = ".......X..";
    char l7[] = ".X.....X..";
    char l8[] = ".X........";
    char l9[] = "..........";
    char *world[] = {l0, l1, l2, l3, l4, l5, l6, l7, l8, l9, 0};

    run_world("big_map", world);
}

static void case_ten_islands(void)
{
    char l0[] = "X.X.X.X.X.";
    char l1[] = "..........";
    char l2[] = ".X.X.X.X.X";
    char *world[] = {l0, l1, l2, 0};

    run_world("ten_islands", world);
}

int main(void)
{
    case_empty_world();
    case_blank_world();
    case_one_island();
    case_two_islands();
    case_diagonal_trap();
    case_l_shape();
    case_single_x();
    case_big_map();
    case_ten_islands();
    return (0);
}
