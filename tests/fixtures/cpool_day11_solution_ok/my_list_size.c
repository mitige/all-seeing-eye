/*
** EPITECH PROJECT, 2026
** cpool_day11
** File description:
** my_list_size
*/

#include <stddef.h>
#include "mylist.h"

int my_list_size(linked_list_t const *begin)
{
    int size = 0;

    while (begin != NULL) {
        size++;
        begin = begin->next;
    }
    return (size);
}
