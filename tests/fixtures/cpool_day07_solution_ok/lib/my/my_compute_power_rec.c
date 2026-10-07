/*
** EPITECH PROJECT, 2026
** cpool_day07
** File description:
** my_compute_power_rec
*/

int my_compute_power_rec(int nb, int power)
{
    if (power < 0) {
        return (0);
    }
    if (power == 0) {
        return (1);
    }
    return (nb * my_compute_power_rec(nb, power - 1));
}
