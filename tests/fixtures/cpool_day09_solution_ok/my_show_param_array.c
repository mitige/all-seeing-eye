/*
** EPITECH PROJECT, 2026
** cpool_day09
** File description:
** my_show_param_array
*/

int my_putstr(char const *str);
void my_putchar(char c);
int my_put_nbr(int nb);
int my_show_word_array(char *const *tab);

struct info_param
{
    int length;
    char *str;
    char *copy;
    char **word_array;
};

int my_show_param_array(struct info_param const *par)
{
    int i = 0;

    while (par[i].str != 0) {
        my_putstr(par[i].str);
        my_putchar('\n');
        my_put_nbr(par[i].length);
        my_putchar('\n');
        my_show_word_array(par[i].word_array);
        i++;
    }
    return (0);
}
