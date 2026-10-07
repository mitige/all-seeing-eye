/*
** EPITECH PROJECT, 2026
** cpool_day11
** File description:
** main de test pour my_find_node
*/

/* « two » apparaît deux fois : c'est le PREMIER nœud qui doit être
** rendu — on affiche sa data puis celle de son next (« two » puis
** « three » ; le second « two » serait suivi de NULL). Puis une
** data_ref absente : NULL attendu.
*/

#include <stdio.h>
#include "list_common.h"
#include "prototypes.h"

int main(void)
{
    char *items[] = {"one", "two", "three", "two"};
    linked_list_t *list = ll_build(items, 4);
    linked_list_t *found = my_find_node(list, "two", &ll_cmp_str);

    if (found == NULL) {
        printf("NULL\n");
    } else {
        printf("%s\n", (char *)found->data);
        if (found->next != NULL)
            printf("%s\n", (char *)found->next->data);
        else
            printf("NULL\n");
    }
    found = my_find_node(list, "absent", &ll_cmp_str);
    printf("%s\n", found == NULL ? "NULL" : "FOUND");
    ll_free(list);
    return (0);
}
