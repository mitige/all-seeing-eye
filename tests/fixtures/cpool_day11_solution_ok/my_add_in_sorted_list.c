/*
** EPITECH PROJECT, 2026
** cpool_day11
** File description:
** my_add_in_sorted_list
*/

#include <stddef.h>
#include <stdlib.h>
#include "mylist.h"

void my_add_in_sorted_list(linked_list_t **begin, void *data,
    int (*cmp)(void *, void *))
{
    linked_list_t *node = malloc(sizeof(*node));

    if (node == NULL)
        return;
    node->data = data;
    while (*begin != NULL && (*cmp)((*begin)->data, data) < 0)
        begin = &(*begin)->next;
    node->next = *begin;
    *begin = node;
}
