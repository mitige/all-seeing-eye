/*
** EPITECH PROJECT, 2026
** cpool_day09
** File description:
** my_params_to_array
*/

#include <stdlib.h>

int my_strlen(char const *str);
char *my_strdup(char const *src);
char **my_str_to_word_array(char const *str);

struct info_param
{
    int length;
    char *str;
    char *copy;
    char **word_array;
};

struct info_param *my_params_to_array(int ac, char **av)
{
    struct info_param *params = malloc(sizeof(*params) * (ac + 1));
    int i = 0;

    if (params == NULL)
        return (NULL);
    while (i < ac) {
        params[i].length = my_strlen(av[i]);
        params[i].str = av[i];
        params[i].copy = my_strdup(av[i]);
        params[i].word_array = my_str_to_word_array(av[i]);
        i++;
    }
    params[ac].str = NULL;
    return (params);
}
