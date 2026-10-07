/*
** EPITECH PROJECT, 2026
** cpool_day10 battery
** File description:
** main_my_sort_word_array.c — harness de my_sort_word_array :
** plusieurs tableaux (général, singleton, vide, casse/chiffres),
** un marqueur "===" par cas, "RET" si le retour n'est pas 0.
*/

#include <stdio.h>

int my_sort_word_array(char **tab);

static void show(char **tab)
{
    int i = 0;

    while (tab[i] != NULL) {
        printf("%s\n", tab[i]);
        i++;
    }
    printf("===\n");
}

static void run_case(char **tab, int tag)
{
    if (my_sort_word_array(tab) != 0)
        printf("RET%d\n", tag);
    show(tab);
}

int main(void)
{
    char *t1[] = {"banana", "apple", "cherry", NULL};
    char *t2[] = {"a", NULL};
    char *t3[] = {NULL};
    char *t4[] = {"zzz", "AAA", "abc", "000", "apple", NULL};

    run_case(t1, 1);
    run_case(t2, 2);
    run_case(t3, 3);
    run_case(t4, 4);
    return (0);
}
