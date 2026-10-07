/*
** EPITECH PROJECT, 2026
** cpool_workshoplib
** File description:
** my_is_prime
*/

int my_is_prime(int nb)
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
