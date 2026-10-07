/*
** EPITECH PROJECT, 2026
** cpool_day11
** File description:
** main de test pour my_delete_nodes
*/

/* Quatre cas : toutes les occurrences au milieu (« x » trois fois),
** une data_ref absente (liste inchangée), la suppression en tête,
** puis vidage complet (NULL attendu). Les nœuds retirés sont libérés
** par l'étudiant ; le harness ne libère que les survivants. La valeur
** de retour n'est PAS testée : le sujet ne la décrit pas.
*/

#include <stdio.h>
#include "list_common.h"
#include "prototypes.h"

int main(void)
{
    char *items[] = {"a", "x", "b", "x", "c", "x"};
    linked_list_t *list = ll_build(items, 6);

    my_delete_nodes(&list, "x", &ll_cmp_str);
    ll_show(list);
    printf("===\n");
    my_delete_nodes(&list, "absent", &ll_cmp_str);
    ll_show(list);
    printf("===\n");
    my_delete_nodes(&list, "a", &ll_cmp_str);
    ll_show(list);
    printf("===\n");
    my_delete_nodes(&list, "b", &ll_cmp_str);
    my_delete_nodes(&list, "c", &ll_cmp_str);
    if (list == NULL)
        printf("NULL\n");
    else
        ll_show(list);
    ll_free(list);
    return (0);
}
