/*
** EPITECH PROJECT, 2026
** cpool_day09
** File description:
** main de test pour swap_endian_color
*/

/*
** Couleur ARGB -> inversion complète de l'ordre des 4 octets.
** 0x000000FF devient 0xFF000000 (-16777216 en int signé) et
** réciproquement : un swap partiel ou un simple R<->B est détecté.
*/

#include <stdio.h>

int swap_endian_color(int color);

int main(void)
{
    printf("%d\n", swap_endian_color(0));
    printf("%d\n", swap_endian_color(0x11223344));
    printf("%d\n", swap_endian_color(255));
    printf("%d\n", swap_endian_color(-16777216));
    printf("%d\n", swap_endian_color(-1));
    return (0);
}
