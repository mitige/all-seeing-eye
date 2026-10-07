/*
** EPITECH PROJECT, 2026
** cpool_day11
** File description:
** main de test pour my_list_size
*/

/* Tailles de 3, 1 et 0 (liste NULL) — une taille par ligne. */

#include <stdio.h>
#include "list_common.h"
#include "prototypes.h"

int main(void)
{
    char *items[] = {"a", "b", "c"};
    linked_list_t *list = ll_build(items, 3);

    printf("%d\n", my_list_size(list));
    ll_free(list);
    list = ll_build(items, 1);
    printf("%d\n", my_list_size(list));
    ll_free(list);
    printf("%d\n", my_list_size(NULL));
    return (0);
}
