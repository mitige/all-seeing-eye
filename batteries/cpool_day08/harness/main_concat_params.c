/*
** EPITECH PROJECT, 2026
** cpool_day08
** File description:
** main de test pour concat_params
*/

#include <stdlib.h>
#include <string.h>
#include <unistd.h>

char *concat_params(int argc, char **argv);

static void show(int argc, char **argv)
{
    char *out = concat_params(argc, argv);

    if (out == NULL) {
        write(1, "NULL\n", 5);
        return;
    }
    write(1, out, strlen(out));
    write(1, "\n===\n", 5);
}

int main(void)
{
    char *case1[] = {"./concat_params", "toto", "titi", NULL};
    char *case2[] = {"lonely", NULL};
    char *case3[] = {"a", "", "b", NULL};

    show(3, case1);
    show(1, case2);
    show(3, case3);
    return (0);
}
