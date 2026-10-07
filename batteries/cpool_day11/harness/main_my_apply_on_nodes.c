/*
** EPITECH PROJECT, 2026
** cpool_day11
** File description:
** main de test pour my_apply_on_nodes
*/

/* f = ll_print : chaque data affichée, dans l'ordre de la liste (un
** oubli ou une inversion se voit). Cas liste NULL ensuite — END borne
** le cas qui ne doit rien afficher. La valeur de retour n'est PAS
** testée : le sujet annonce un int sans jamais le décrire.
*/

#include <stdio.h>
#include "list_common.h"
#include "prototypes.h"

int main(void)
{
    char *items[] = {"un", "deux", "trois"};
    linked_list_t *list = ll_build(items, 3);

    my_apply_on_nodes(list, &ll_print);
    printf("===\n");
    my_apply_on_nodes(NULL, &ll_print);
    printf("END\n");
    ll_free(list);
    return (0);
}
