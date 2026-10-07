/*
** EPITECH PROJECT, 2026
** cpool_day10 battery
** File description:
** main_my_advanced_sort_word_array.c — harness de
** my_advanced_sort_word_array : cmp façon my_strcmp (doit donner le
** même résultat que my_sort_word_array), cmp par longueur (toutes
** distinctes — pas d'égalité, la stabilité n'est pas exigée), cmp
** lexicographique inversé, tableau vide.
*/

#include <stdio.h>

int my_advanced_sort_word_array(char **tab,
    int (*cmp)(char const *, char const *));

static int cmp_str(char const *a, char const *b)
{
    int i = 0;

    while (a[i] != '\0' && a[i] == b[i])
        i++;
    return ((unsigned char)a[i] - (unsigned char)b[i]);
}

static int slen(char const *s)
{
    int n = 0;

    while (s[n] != '\0')
        n++;
    return (n);
}

static int cmp_len(char const *a, char const *b)
{
    return (slen(a) - slen(b));
}

static int cmp_rev(char const *a, char const *b)
{
    return (cmp_str(b, a));
}

static void show(char **tab)
{
    int i = 0;

    while (tab[i] != NULL) {
        printf("%s\n", tab[i]);
        i++;
    }
    printf("===\n");
}

int main(void)
{
    char *t1[] = {"banana", "apple", "cherry", NULL};
    char *t2[] = {"bbbb", "a", "ccc", "dd", NULL};
    char *t3[] = {"apple", "banana", "cherry", NULL};
    char *t4[] = {NULL};

    if (my_advanced_sort_word_array(t1, &cmp_str) != 0)
        printf("RET1\n");
    show(t1);
    if (my_advanced_sort_word_array(t2, &cmp_len) != 0)
        printf("RET2\n");
    show(t2);
    if (my_advanced_sort_word_array(t3, &cmp_rev) != 0)
        printf("RET3\n");
    show(t3);
    if (my_advanced_sort_word_array(t4, &cmp_str) != 0)
        printf("RET4\n");
    show(t4);
    return (0);
}
