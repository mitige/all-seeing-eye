/*
** EPITECH PROJECT, 2026
** cpool_day11
** File description:
** list_cmp.c — comparateurs et fonction d'application des harness
*/

/* Comparateurs strcmp dans les deux formats du sujet (le « cmp could
** be my_strcmp » : égalité ssi cmp renvoie 0) — void const * pour
** find/delete/apply_matching, void * pour sort/add_in_sorted/merge —
** et ll_print, le f des my_apply_on_* (affiche la data, un élément
** par ligne).
*/

#include <stdio.h>
#include <string.h>
#include "list_common.h"

int ll_cmp_str(void *data, void const *ref)
{
    return (strcmp((char const *)data, (char const *)ref));
}

int ll_cmp_str_vv(void *a, void *b)
{
    return (strcmp((char const *)a, (char const *)b));
}

int ll_print(void *data)
{
    printf("%s\n", (char const *)data);
    return (0);
}
