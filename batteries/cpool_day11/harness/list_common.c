/*
** EPITECH PROJECT, 2026
** cpool_day11
** File description:
** list_common.c — construction, affichage et libération des listes
*/

/* Les données manipulées sont des chaînes : affichage un élément par
** ligne. ll_build(items, n) construit la liste items[0] en tête.
*/

#include <stdio.h>
#include <stdlib.h>
#include "list_common.h"

linked_list_t *ll_new_node(void *data)
{
    linked_list_t *node = malloc(sizeof(*node));

    if (node == NULL)
        return (NULL);
    node->data = data;
    node->next = NULL;
    return (node);
}

void ll_push(linked_list_t **begin, void *data)
{
    linked_list_t *node = ll_new_node(data);

    if (node == NULL)
        return;
    node->next = *begin;
    *begin = node;
}

linked_list_t *ll_build(char *const *items, int count)
{
    linked_list_t *list = NULL;
    int i = count - 1;

    while (i >= 0) {
        ll_push(&list, items[i]);
        i--;
    }
    return (list);
}

void ll_show(linked_list_t const *begin)
{
    while (begin != NULL) {
        printf("%s\n", (char const *)begin->data);
        begin = begin->next;
    }
}

void ll_free(linked_list_t *begin)
{
    linked_list_t *tmp;

    while (begin != NULL) {
        tmp = begin->next;
        free(begin);
        begin = tmp;
    }
}
