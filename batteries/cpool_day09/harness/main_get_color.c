/*
** EPITECH PROJECT, 2026
** cpool_day09
** File description:
** main de test pour get_color
*/

/*
** RGB -> int : R en bits 16-23, G en bits 8-15, B en bits 0-7.
** Un décalage erroné (ex. red << 24) est détecté dès 255/0/0.
*/

#include <stdio.h>

int get_color(unsigned char red, unsigned char green, unsigned char blue);

int main(void)
{
    printf("%d\n", get_color(0, 0, 0));
    printf("%d\n", get_color(255, 255, 255));
    printf("%d\n", get_color(255, 0, 0));
    printf("%d\n", get_color(0, 255, 0));
    printf("%d\n", get_color(0, 0, 255));
    printf("%d\n", get_color(18, 52, 86));
    return (0);
}
