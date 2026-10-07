/*
** EPITECH PROJECT, 2026
** cpool_day11
** File description:
** list_common.h — helpers partagés des harness
*/

/* Helpers de construction, d'affichage, de libération et de
** comparaison des listes de chaînes manipulées par les harness. Les
** noms sont préfixés ll_ : jamais de collision avec une fonction my_*
** de la delivery compilée dans le même binaire de test.
*/

#ifndef LIST_COMMON_H
    #define LIST_COMMON_H

    #include "mylist.h"

linked_list_t *ll_new_node(void *data);
void ll_push(linked_list_t **begin, void *data);
linked_list_t *ll_build(char *const *items, int count);
void ll_show(linked_list_t const *begin);
void ll_free(linked_list_t *begin);
int ll_cmp_str(void *data, void const *ref);
int ll_cmp_str_vv(void *a, void *b);
int ll_print(void *data);

#endif
