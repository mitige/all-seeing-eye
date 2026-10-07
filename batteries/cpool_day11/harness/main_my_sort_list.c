/*
** EPITECH PROJECT, 2026
** cpool_day11
** File description:
** main de test pour my_sort_list
*/

/* Tri croissant par cmp (strcmp) : une liste mélangée, puis la même
** déjà triée (un tri qui suppose l'entrée mélangée doit rester
** correct), une liste d'un seul élément, une liste NULL.
*/

#include <stdio.h>
#include "list_common.h"
#include "prototypes.h"

static void sort_and_show(linked_list_t **list)
{
    my_sort_list(list, &ll_cmp_str_vv);
    ll_show(*list);
    printf("===\n");
}

int main(void)
{
    char *items[] = {"pear", "apple", "orange", "banana", "kiwi"};
    char *one[] = {"solo"};
    linked_list_t *list = ll_build(items, 5);
    linked_list_t *single = ll_build(one, 1);
    linked_list_t *empty = NULL;

    sort_and_show(&list);
    sort_and_show(&list);
    sort_and_show(&single);
    my_sort_list(&empty, &ll_cmp_str_vv);
    if (empty == NULL)
        printf("NULL\n");
    else
        ll_show(empty);
    ll_free(list);
    ll_free(single);
    return (0);
}
