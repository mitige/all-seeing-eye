/*
** EPITECH PROJECT, 2026
** cpool_day09
** File description:
** my_put_nbr de référence (libmy de la batterie)
*/

void my_putchar(char c);

int my_put_nbr(int nb)
{
    long n = nb;

    if (n < 0) {
        my_putchar('-');
        n = -n;
    }
    if (n >= 10)
        my_put_nbr(n / 10);
    my_putchar('0' + n % 10);
    return (0);
}
