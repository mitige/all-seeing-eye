/*
** EPITECH PROJECT, 2026
** cpool_day09
** File description:
** main de test pour my.h
*/

/*
** Vérifie que le header rendu est auto-suffisant (include guard,
** prototypes utilisables) en appelant le cœur canonique de libmy :
** my_putchar, my_putstr, my_put_nbr, my_strlen — définis par la lib
** de référence de la batterie (extra_sources).
*/

#include "my.h"

int main(void)
{
    my_putstr("my.h ok: ");
    my_put_nbr(my_strlen("piscine"));
    my_putchar('\n');
    return (0);
}
