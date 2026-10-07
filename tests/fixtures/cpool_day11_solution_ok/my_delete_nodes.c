/*
** EPITECH PROJECT, 2026
** cpool_day11
** File description:
** my_delete_nodes
*/

#include <stddef.h>
#include <stdlib.h>
#include "mylist.h"

int my_delete_nodes(linked_list_t **begin, void const *data_ref,
    int (*cmp)(void *, void const *))
{
    linked_list_t **cur = begin;
    linked_list_t *tmp;

    while (*cur != NULL) {
        if ((*cmp)((*cur)->data, data_ref) == 0) {
            tmp = *cur;
            *cur = (*cur)->next;
            free(tmp);
        } else {
            cur = &(*cur)->next;
        }
    }
    return (0);
}
