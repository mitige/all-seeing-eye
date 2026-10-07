/*
** EPITECH PROJECT, 2026
** cpool_day11
** File description:
** main de test pour my_apply_on_matching_nodes
*/

/* « apple » apparaît deux fois : seules ces deux lignes doivent être
** affichées (un test d'égalité inversé afficherait banana et cherry
** — le sujet est formel : égaux ssi cmp renvoie 0). Puis une
** data_ref absente — END borne le cas qui ne doit rien afficher. La
** valeur de retour n'est PAS testée : le sujet ne la décrit pas.
*/

#include <stdio.h>
#include "list_common.h"
#include "prototypes.h"

int main(void)
{
    char *items[] = {"apple", "banana", "apple", "cherry"};
    linked_list_t *list = ll_build(items, 4);

    my_apply_on_matching_nodes(list, &ll_print, "apple", &ll_cmp_str);
    printf("===\n");
    my_apply_on_matching_nodes(list, &ll_print, "kiwi", &ll_cmp_str);
    printf("END\n");
    ll_free(list);
    return (0);
}
