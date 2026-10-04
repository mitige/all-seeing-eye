/*
** EPITECH PROJECT, 2026
** cpool_day03
** File description:
** my_put_nbr
*/

void my_putchar(char c);

static void print_unsigned(unsigned int n)
{
    if (n > 9) {
        print_unsigned(n / 10);
    }
    my_putchar('0' + (int)(n % 10));
}

int my_put_nbr(int nb)
{
    unsigned int n;

    if (nb < 0) {
        my_putchar('-');
        n = 0 - (unsigned int)nb;
    } else {
        n = (unsigned int)nb;
    }
    print_unsigned(n);
    return (0);
}
