/*
** EPITECH PROJECT, 2026
** cpool_day03
** File description:
** main de test pour my_put_nbr
*/

int my_put_nbr(int nb);

int main(void)
{
    my_put_nbr(0);
    my_put_nbr(42);
    my_put_nbr(-2147483647 - 1);
    my_put_nbr(2147483647);
    return (0);
}
