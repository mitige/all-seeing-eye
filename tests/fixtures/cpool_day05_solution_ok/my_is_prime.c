/*
** EPITECH PROJECT, 2026
** cpool_day05
** File description:
** my_is_prime
*/

int my_is_prime(int nb)
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
