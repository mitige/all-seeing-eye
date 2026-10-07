/*
** EPITECH PROJECT, 2026
** cpool_day11
** File description:
** my_apply_on_matching_nodes
*/

#include <stddef.h>
#include "mylist.h"

int my_apply_on_matching_nodes(linked_list_t *begin, int (*f)(void *),
    void const *data_ref, int (*cmp)(void *, void const *))
{
    while (begin != NULL) {
        if ((*cmp)(begin->data, data_ref) == 0)
            (*f)(begin->data);
        begin = begin->next;
    }
    return (0);
}
