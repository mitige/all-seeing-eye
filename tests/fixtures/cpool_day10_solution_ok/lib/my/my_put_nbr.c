/*
** EPITECH PROJECT, 2026
** cpool_day10
** File description:
** my_put_nbr.c
*/

#include "my.h"

int my_put_nbr(int nb)
{
    long n = nb;
    char c;

    if (n < 0) {
        my_putchar('-');
        n = -n;
    }
    if (n >= 10)
        my_put_nbr((int)(n / 10));
    c = (char)('0' + n % 10);
    my_putchar(c);
    return (0);
}
