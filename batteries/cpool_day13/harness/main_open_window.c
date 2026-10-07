/*
** EPITECH PROJECT, 2026
** all-seeing-eye
** File description:
** Harness open_window (Task 01 du sujet) : le main de la delivery,
** renommé cpool_d13_student_main via -Dmain= (pattern cpool_day07),
** n'est JAMAIS appelé — ouvrir une fenêtre est impossible en salle
** blanche headless et « keep it open » bouclerait jusqu'au timeout.
** Le test prouve : la delivery existe (prelim), compile -Wall -Wextra,
** et se lie à la VRAIE libcsfml-graphics (toutes ses références
** sfRenderWindow_* doivent résoudre). Limite documentée dans le TOML :
** un main vide passerait — le comportement fenêtre n'est pas
** vérifiable sans display.
*/

#include <stdio.h>

#undef main

int main(void)
{
    printf("OK\n");
    return (0);
}
