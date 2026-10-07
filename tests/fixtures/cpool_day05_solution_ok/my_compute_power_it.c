/*
** EPITECH PROJECT, 2026
** cpool_day05
** File description:
** my_compute_power_it
*/

int my_compute_power_it(int nb, int p)
{
    long result = 1;
    int i = 0;

    if (p < 0)
        return (0);
    while (i < p) {
        result = result * nb;
        if (result > 2147483647 || result < -2147483647 - 1)
            return (0);
        i = i + 1;
    }
    return ((int)result);
}
