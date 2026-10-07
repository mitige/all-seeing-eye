/*
** EPITECH PROJECT, 2026
** cpool_countisland
** File description:
** my_find_prime_sup
*/

static int is_prime_here(int nb)
{
    int i = 2;

    if (nb < 2) {
        return (0);
    }
    while (i <= nb / i) {
        if (nb % i == 0) {
            return (0);
        }
        i = i + 1;
    }
    return (1);
}

int my_find_prime_sup(int nb)
{
    if (nb < 2) {
        return (2);
    }
    while (!is_prime_here(nb)) {
        nb = nb + 1;
    }
    return (nb);
}
