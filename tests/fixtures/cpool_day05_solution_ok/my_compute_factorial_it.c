/*
** EPITECH PROJECT, 2026
** cpool_day05
** File description:
** my_compute_factorial_it
*/

int my_compute_factorial_it(int nb)
{
    long result = 1;
    int i = 2;

    if (nb < 0)
        return (0);
    while (i <= nb) {
        result = result * i;
        if (result > 2147483647)
            return (0);
        i = i + 1;
    }
    return ((int)result);
}
