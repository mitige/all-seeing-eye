/*
** EPITECH PROJECT, 2026
** cpool_day09
** File description:
** get_color
*/

int get_color(unsigned char red, unsigned char green, unsigned char blue)
{
    return ((red << 16) | (green << 8) | blue);
}
