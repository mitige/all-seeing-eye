/*
** EPITECH PROJECT, 2026
** cpool_day03
** File description:
** my_put_nbr
*/

/* Fixture KO volontaire : le débordement sur INT_MIN repose sur un
** UB signé assumé (`nb = -nb`) — si -fsanitize=undefined est ajouté
** un jour à la chaîne, le verdict devient crashed au lieu de failed.
*/

void my_putchar(char c);

int my_put_nbr(int nb)
{
    if (nb < 0) {
        my_putchar('-');
        nb = -nb;
    }
    if (nb > 9) {
        my_put_nbr(nb / 10);
    }
    my_putchar('0' + nb % 10);
    return (0);
}
