/*
** EPITECH PROJECT, 2026
** cpool_day11
** File description:
** main de test pour my_add_in_sorted_list
*/

/* Cinq insertions en partant d'une liste NULL : création, queue,
** tête, milieu, puis « apricot » (juste après « apple » — un élément
** inséré une position trop loin se verrait). La liste est affichée
** après CHAQUE insertion : la première insertion fautive est ainsi
** visible dans le diff.
*/

#include <stdio.h>
#include "list_common.h"
#include "prototypes.h"

int main(void)
{
    linked_list_t *list = NULL;

    my_add_in_sorted_list(&list, "banana", &ll_cmp_str_vv);
    ll_show(list);
    printf("===\n");
    my_add_in_sorted_list(&list, "orange", &ll_cmp_str_vv);
    ll_show(list);
    printf("===\n");
    my_add_in_sorted_list(&list, "apple", &ll_cmp_str_vv);
    ll_show(list);
    printf("===\n");
    my_add_in_sorted_list(&list, "kiwi", &ll_cmp_str_vv);
    ll_show(list);
    printf("===\n");
    my_add_in_sorted_list(&list, "apricot", &ll_cmp_str_vv);
    ll_show(list);
    ll_free(list);
    return (0);
}
