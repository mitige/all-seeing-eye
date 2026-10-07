/*
** EPITECH PROJECT, 2026
** cpool_day11
** File description:
** main de test pour my_params_to_list
*/

/* argv SYNTHÉTIQUE (« ./a.out » fixe — le vrai av[0] du binaire de
** test serait le chemin de la salle blanche, variable). Sujet : avec
** ./a.out test arg2 arg3, le scan de la liste donne arg3, arg2, test,
** ./a.out (chaque argument empilé en tête). Le cas ac = 0 doit rendre
** une liste NULL.
*/

#include <stdio.h>
#include "list_common.h"
#include "prototypes.h"

int main(void)
{
    char *av[] = {"./a.out", "test", "arg2", "arg3"};
    linked_list_t *list = my_params_to_list(4, av);

    ll_show(list);
    ll_free(list);
    printf("===\n");
    list = my_params_to_list(0, av);
    if (list == NULL)
        printf("NULL\n");
    else
        ll_show(list);
    ll_free(list);
    return (0);
}
