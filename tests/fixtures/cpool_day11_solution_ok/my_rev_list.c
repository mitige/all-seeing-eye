/*
** EPITECH PROJECT, 2026
** cpool_day11
** File description:
** my_rev_list
*/

#include <stddef.h>
#include "mylist.h"

void my_rev_list(linked_list_t **begin)
{
    linked_list_t *prev = NULL;
    linked_list_t *next;
    linked_list_t *cur;

    if (begin == NULL)
        return;
    cur = *begin;
    while (cur != NULL) {
        next = cur->next;
        cur->next = prev;
        prev = cur;
        cur = next;
    }
    *begin = prev;
}
