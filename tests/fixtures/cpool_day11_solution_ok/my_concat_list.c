/*
** EPITECH PROJECT, 2026
** cpool_day11
** File description:
** my_concat_list
*/

#include <stddef.h>
#include "mylist.h"

void my_concat_list(linked_list_t **begin1, linked_list_t *begin2)
{
    linked_list_t *node;

    if (begin2 == NULL)
        return;
    if (*begin1 == NULL) {
        *begin1 = begin2;
        return;
    }
    node = *begin1;
    while (node->next != NULL)
        node = node->next;
    node->next = begin2;
}
