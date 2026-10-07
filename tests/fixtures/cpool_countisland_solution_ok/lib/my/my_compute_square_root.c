/*
** EPITECH PROJECT, 2026
** cpool_countisland
** File description:
** my_compute_square_root
*/

int my_compute_square_root(int nb)
{
    long int i = 1;

    if (nb <= 0) {
        return (0);
    }
    while (i * i < nb) {
        i = i + 1;
    }
    if (i * i == nb) {
        return ((int)i);
    }
    return (0);
}
