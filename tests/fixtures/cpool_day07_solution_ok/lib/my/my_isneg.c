/*
** EPITECH PROJECT, 2026
** cpool_day07
** File description:
** my_isneg
*/

#include <unistd.h>

static void put_c(char c)
{
    write(1, &c, 1);
}

int my_isneg(int nb)
{
    if (nb < 0) {
        put_c('N');
    } else {
        put_c('P');
    }
    return (0);
}
