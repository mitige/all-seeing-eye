/*
** EPITECH PROJECT, 2026
** cpool_day11
** File description:
** main de test pour my_concat_list
*/

/* Le sujet interdit de CRÉER des éléments (« You must link the two
** lists together ») : le harness retient l'adresse du premier nœud de
** begin2 avant l'appel et vérifie après que le 3e nœud de begin1 EST
** ce nœud (LINKED — un étudiant qui allouerait des copies afficherait
** COPIED). Puis concaténation sur une liste NULL, puis concaténation
** d'une liste NULL (begin1 inchangée).
*/

#include <stdio.h>
#include "list_common.h"
#include "prototypes.h"

int main(void)
{
    char *items1[] = {"a", "b"};
    char *items2[] = {"c", "d"};
    char *one[] = {"e"};
    linked_list_t *l1 = ll_build(items1, 2);
    linked_list_t *l2 = ll_build(items2, 2);
    linked_list_t *c_node = l2;
    linked_list_t *empty = NULL;

    my_concat_list(&l1, l2);
    ll_show(l1);
    printf("%s\n", l1->next->next == c_node ? "LINKED" : "COPIED");
    printf("===\n");
    my_concat_list(&empty, ll_build(one, 1));
    ll_show(empty);
    ll_free(empty);
    printf("===\n");
    my_concat_list(&l1, NULL);
    ll_show(l1);
    ll_free(l1);
    return (0);
}
