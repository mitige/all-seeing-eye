/*
** EPITECH PROJECT, 2026
** cpool_day11
** File description:
** main de test pour my_rev_list
*/

/* Renversement d'une liste de 4 (l'ordre complet est affiché après
** l'appel : une inversion partielle ou un simple swap des extrémités
** se voit tout de suite), d'une liste d'un seul élément, et d'une
** liste NULL.
*/

#include <stdio.h>
#include "list_common.h"
#include "prototypes.h"

int main(void)
{
    char *items[] = {"a", "b", "c", "d"};
    char *one[] = {"solo"};
    linked_list_t *list = ll_build(items, 4);
    linked_list_t *single = ll_build(one, 1);
    linked_list_t *empty = NULL;

    my_rev_list(&list);
    ll_show(list);
    printf("===\n");
    my_rev_list(&single);
    ll_show(single);
    printf("===\n");
    my_rev_list(&empty);
    if (empty == NULL)
        printf("NULL\n");
    else
        ll_show(empty);
    ll_free(list);
    ll_free(single);
    return (0);
}
