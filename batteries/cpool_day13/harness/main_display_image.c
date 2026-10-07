/*
** EPITECH PROJECT, 2026
** all-seeing-eye
** File description:
** Harness display_image (Task 04 du sujet) : même traitement que
** open_window — le main de la delivery est renommé via -Dmain= et
** jamais appelé (affichage impossible en headless). Le test prouve
** existence (prelim) + compile + link réel contre libcsfml-graphics :
** les références sfTexture_*, sfSprite_* et sfRenderWindow_* de la
** delivery doivent résoudre.
*/

#include <stdio.h>

#undef main

int main(void)
{
    printf("OK\n");
    return (0);
}
