/*
** EPITECH PROJECT, 2026
** cpool_day11
** File description:
** prototypes.h — les 11 fonctions du sujet
*/

/* Prototypes EXACTS du sujet Day11, centralisés : les mains des
** harness incluent ce header au lieu de déclarer dans le .c (C-H1).
** Seule liberté d'écriture : « char *const *av » sans l'espace avant
** const — type strictement identique au « char * const *av » du
** sujet, mais épargne la C-V3 à chaque compilation de harness.
** Déclarations seules : chaque main n'appelle que SA fonction, le
** link ne requiert que la delivery correspondante.
*/

#ifndef PROTOTYPES_H
    #define PROTOTYPES_H

    #include "mylist.h"

linked_list_t *my_params_to_list(int ac, char *const *av);
int my_list_size(linked_list_t const *begin);
void my_rev_list(linked_list_t **begin);
int my_apply_on_nodes(linked_list_t *begin, int (*f)(void *));
int my_apply_on_matching_nodes(linked_list_t *begin, int (*f)(void *),
    void const *data_ref, int (*cmp)(void *, void const *));
linked_list_t *my_find_node(linked_list_t const *begin,
    void const *data_ref, int (*cmp)(void *, void const *));
int my_delete_nodes(linked_list_t **begin, void const *data_ref,
    int (*cmp)(void *, void const *));
void my_concat_list(linked_list_t **begin1, linked_list_t *begin2);
void my_sort_list(linked_list_t **begin, int (*cmp)(void *, void *));
void my_add_in_sorted_list(linked_list_t **begin, void *data,
    int (*cmp)(void *, void *));
void my_merge(linked_list_t **begin1, linked_list_t *begin2,
    int (*cmp)(void *, void *));

#endif
