/*
** EPITECH PROJECT, 2026
** cpool_day08
** File description:
** my_show_word_array
*/

#include <stddef.h>
#include <unistd.h>

static void show_word(char const *word)
{
    int len = 0;

    while (word[len] != '\0') {
        len = len + 1;
    }
    write(1, word, len);
}

int my_show_word_array(char * const *tab)
{
    int i = 0;

    while (tab[i] != NULL) {
        show_word(tab[i]);
        write(1, "\n", 1);
        i = i + 1;
    }
    return (0);
}
