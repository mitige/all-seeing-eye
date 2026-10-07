/*
** EPITECH PROJECT, 2026
** cpool_day05
** File description:
** my_find_prime_sup
*/

static int is_prime(int nb)
{
    long i = 2;

    if (nb < 2)
        return (0);
    while (i * i <= nb) {
        if (nb % (int)i == 0)
            return (0);
        i = i + 1;
    }
    return (1);
}

int my_find_prime_sup(int nb)
{
    if (nb <= 2)
        return (2);
    while (!is_prime(nb))
        nb = nb + 1;
    return (nb);
}
