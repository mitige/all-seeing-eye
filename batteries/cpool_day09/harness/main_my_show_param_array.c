/*
** EPITECH PROJECT, 2026
** cpool_day09
** File description:
** main de test pour my_show_param_array
*/

/*
** La struct info_param est celle du sujet, à l'identique (« the tests
** set will use its own ») — définie AVANT les prototypes qui la
** référencent. Le tableau est construit par le harness avec la lib de
** référence (my_strlen, my_strdup, my_str_to_word_array) — comme la
** vraie moulinette qui bâtit le sien via la libmy de l'étudiant.
** argv synthétique, comme pour my_params_to_array (déterminisme de
** la salle blanche). Affichage attendu par cellule : paramètre,
** taille, mots (un par ligne).
*/

struct info_param
{
    int length;        // parameter's length
    char *str;         // parameter's address
    char *copy;        // parameter's copy
    char **word_array; // the result of my_str_to_word_array(str)
};

int my_show_param_array(struct info_param const *par);
int my_strlen(char const *str);
char *my_strdup(char const *src);
char **my_str_to_word_array(char const *str);

static void fill_cell(struct info_param *cell, char *str)
{
    cell->length = my_strlen(str);
    cell->str = str;
    cell->copy = my_strdup(str);
    cell->word_array = my_str_to_word_array(str);
}

int main(void)
{
    char *av[] = {"./a.out", "hello world", "  abc  def  ", "x", 0};
    struct info_param params[5];
    int i = 0;

    while (i < 4) {
        fill_cell(&params[i], av[i]);
        i++;
    }
    params[4].length = 0;
    params[4].str = 0;
    params[4].copy = 0;
    params[4].word_array = 0;
    my_show_param_array(params);
    return (0);
}
