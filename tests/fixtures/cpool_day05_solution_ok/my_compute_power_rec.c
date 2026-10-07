/*
** EPITECH PROJECT, 2026
** cpool_day05
** File description:
** my_compute_power_rec
*/

int my_compute_power_rec(int nb, int p)
{
    long result;

    if (p < 0)
        return (0);
    if (p == 0)
        return (1);
    result = (long)nb * my_compute_power_rec(nb, p - 1);
    if (result > 2147483647 || result < -2147483647 - 1)
        return (0);
    return ((int)result);
}
