/*
** EPITECH PROJECT, 2026
** cpool_day08
** File description:
** main de test pour my_str_to_word_array
*/

#include <stdlib.h>
#include <string.h>
#include <unistd.h>

char **my_str_to_word_array(char const *str);

static void show(char const *str)
{
    char **tab = my_str_to_word_array(str);
    int i = 0;

    if (tab == NULL) {
        write(1, "NULL\n", 5);
        return;
    }
    while (tab[i] != NULL) {
        write(1, tab[i], strlen(tab[i]));
        write(1, "\n", 1);
        i = i + 1;
    }
    write(1, "===\n", 4);
}

int main(void)
{
    show("  Hello\tworld  ");
    show("");
    show("!!!");
    show("a1 b2\tc3\n");
    show("The Answer, please.");
    return (0);
}
