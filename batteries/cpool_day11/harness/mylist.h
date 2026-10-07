/*
** EPITECH PROJECT, 2026
** cpool_day11
** File description:
** mylist.h — la struct du sujet, à l'identique
*/

/* La struct linked_list du sujet reproduite à l'identique (champs,
** ordre, typedef). Copie de la BATTERIE, pas de l'étudiant : pour un
** #include "mylist.h" entre guillemets, le préprocesseur cherche
** d'abord dans le dossier du fichier incluant — les harness compilent
** donc toujours contre CETTE struct, quelle que soit la mylist.h
** rendue (les deliveries de l'étudiant, elles, trouvent la sienne via
** le -Iinclude des cflags).
*/

#ifndef MYLIST_H
    #define MYLIST_H

typedef struct linked_list {
    void *data;
    struct linked_list *next;
} linked_list_t;

#endif
