/*
** EPITECH PROJECT, 2026
** cpool_day10
** File description:
** my_sort_word_array.c
*/

#include <stddef.h>

static int cmp_words(char const *a, char const *b)
{
    int i = 0;

    while (a[i] != '\0' && a[i] == b[i])
        i++;
    return ((unsigned char)a[i] - (unsigned char)b[i]);
}

int my_sort_word_array(char **tab)
{
    int i = 0;
    char *tmp;

    if (tab == NULL || tab[0] == NULL)
        return (0);
    while (tab[i + 1] != NULL) {
        if (cmp_words(tab[i], tab[i + 1]) > 0) {
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
