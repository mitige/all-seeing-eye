/*
** EPITECH PROJECT, 2026
** cpool_day11
** File description:
** my_params_to_list
*/

#include <stdlib.h>
#include "mylist.h"

linked_list_t *my_params_to_list(int ac, char * const *av)
{
    linked_list_t *list = NULL;
    linked_list_t *node;
    int i = 0;

    while (i < ac) {
        node = malloc(sizeof(*node));
        if (node == NULL)
            return (list);
        node->data = av[i];
        node->next = list;
        list = node;
        i++;
    }
    return (list);
}
