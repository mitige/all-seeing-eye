/*
** EPITECH PROJECT, 2026
** cpool_day11
** File description:
** my_merge
*/

#include <stddef.h>
#include "mylist.h"

void my_merge(linked_list_t **begin1, linked_list_t *begin2,
    int (*cmp)(void *, void *))
{
    linked_list_t *next;

    while (begin2 != NULL) {
        next = begin2->next;
        while (*begin1 != NULL && (*cmp)((*begin1)->data, begin2->data) < 0)
            begin1 = &(*begin1)->next;
        begin2->next = *begin1;
        *begin1 = begin2;
        begin2 = next;
    }
}
