/*
** EPITECH PROJECT, 2026
** cpool_day11
** File description:
** main de test pour my_merge
*/

/* Deux listes triées entrelacées, puis fusion d'une liste NULL (rien
** ne change), puis fusion dans une liste NULL (« Watch out for NULL
** pointers! » — begin1 doit devenir la liste triée).
**
** NB : le sort des nœuds de begin2 n'est PAS spécifié (ré-insérés ou
** dupliqués façon my_add_in_sorted_list) — le harness ne libère QUE
** begin1 après l'appel et ne touche plus begin2 : libérer begin2
** serait un double free chez qui ré-insère, et libérer les deux est
** impossible à écrire sans trahir l'une des deux implémentations.
*/

#include <stdio.h>
#include "list_common.h"
#include "prototypes.h"

int main(void)
{
    char *items1[] = {"apple", "orange"};
    char *items2[] = {"banana", "pear"};
    char *items3[] = {"kiwi", "melon"};
    linked_list_t *l1 = ll_build(items1, 2);
    linked_list_t *l2 = ll_build(items2, 2);
    linked_list_t *empty = NULL;

    my_merge(&l1, l2, &ll_cmp_str_vv);
    ll_show(l1);
    printf("===\n");
    my_merge(&l1, NULL, &ll_cmp_str_vv);
    ll_show(l1);
    printf("===\n");
    my_merge(&empty, ll_build(items3, 2), &ll_cmp_str_vv);
    ll_show(empty);
    ll_free(empty);
    ll_free(l1);
    return (0);
}
