/*
** EPITECH PROJECT, 2026
** cpool_day09
** File description:
** main de test pour my_params_to_array
*/

/*
** La struct info_param est celle du sujet, à l'identique (« the tests
** set will use its own »). argv est SYNTHÉTIQUE : le vrai av[0] du
** binaire de test serait le chemin de la salle blanche, variable —
** ici "./a.out" fixe, et l'exigence « including av[0] » reste testée.
** Pour chaque cellule : str, length, copy, puis str APRÈS gribouillage
** de copy[0] (un copy aliasé sur str trahirait le '#'), puis les mots
** via my_show_word_array. « END » confirme la sentinelle str == 0.
*/

int my_putstr(char const *str);
void my_putchar(char c);
int my_put_nbr(int nb);
int my_show_word_array(char *const *tab);

struct info_param
{
    int length;        // parameter's length
    char *str;         // parameter's address
    char *copy;        // parameter's copy
    char **word_array; // the result of my_str_to_word_array(str)
};

struct info_param *my_params_to_array(int ac, char **av);

static void show_cell(struct info_param const *cell)
{
    my_putstr(cell->str);
    my_putchar('\n');
    my_put_nbr(cell->length);
    my_putchar('\n');
    my_putstr(cell->copy);
    my_putchar('\n');
    if (cell->length > 0)
        cell->copy[0] = '#';
    my_putstr(cell->str);
    my_putchar('\n');
    my_show_word_array(cell->word_array);
}

int main(void)
{
    char *av[] = {"./a.out", "hello world", "  abc  def  ", "x", 0};
    struct info_param *params = my_params_to_array(4, av);
    int i = 0;

    while (params[i].str != 0) {
        show_cell(&params[i]);
        i++;
    }
    my_putstr("END\n");
    return (0);
}
