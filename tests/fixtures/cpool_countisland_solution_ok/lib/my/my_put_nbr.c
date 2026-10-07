/*
** EPITECH PROJECT, 2026
** cpool_countisland
** File description:
** my_put_nbr
*/

#include <unistd.h>

static void put_c(char c)
{
    write(1, &c, 1);
}

static void print_unsigned(unsigned int n)
{
    if (n > 9) {
        print_unsigned(n / 10);
    }
    put_c('0' + (int)(n % 10));
}

int my_put_nbr(int nb)
{
    unsigned int n;

    if (nb < 0) {
        put_c('-');
        n = 0 - (unsigned int)nb;
    } else {
        n = (unsigned int)nb;
    }
    print_unsigned(n);
    return (0);
}
