/*
** EPITECH PROJECT, 2026
** cpool_day10
** File description:
** my_advanced_sort_word_array.c
*/

#include <stddef.h>

int my_advanced_sort_word_array(char **tab,
    int (*cmp)(char const *, char const *))
{
    int i = 0;
    char *tmp;

    if (tab == NULL || tab[0] == NULL)
        return (0);
    while (tab[i + 1] != NULL) {
        if (cmp(tab[i], tab[i + 1]) > 0) {
            tmp = tab[i];
            tab[i] = tab[i + 1];
            tab[i + 1] = tmp;
            i = 0;
        } else {
            i++;
        }
    }
    return (0);
}
