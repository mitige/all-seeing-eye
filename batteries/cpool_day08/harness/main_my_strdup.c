/*
** EPITECH PROJECT, 2026
** cpool_day08
** File description:
** main de test pour my_strdup
*/

#include <stdlib.h>
#include <string.h>
#include <unistd.h>

char *my_strdup(char const *src);

static void show(char const *src)
{
    char *dup = my_strdup(src);

    if (dup == NULL) {
        write(1, "NULL\n", 5);
        return;
    }
    write(1, dup, strlen(dup));
    if (dup == src) {
        write(1, " SAME\n", 6);
    } else {
        write(1, "\n", 1);
    }
}

int main(void)
{
    show("hello world");
    show("");
    show("a");
    show("  spaces\tand\ttabs  ");
    return (0);
}
