/*
** EPITECH PROJECT, 2026
** cpool_day11
** File description:
** my_sort_list
*/

#include <stddef.h>
#include "mylist.h"

static int bubble_pass(linked_list_t *list, int (*cmp)(void *, void *))
{
    void *tmp;
    int sorted = 1;

    while (list->next != NULL) {
        if ((*cmp)(list->data, list->next->data) > 0) {
            tmp = list->data;
            list->data = list->next->data;
            list->next->data = tmp;
            sorted = 0;
        }
        list = list->next;
    }
    return (sorted);
}

void my_sort_list(linked_list_t **begin, int (*cmp)(void *, void *))
{
    int sorted = 0;

    if (begin == NULL || *begin == NULL)
        return;
    while (!sorted)
        sorted = bubble_pass(*begin, cmp);
}
