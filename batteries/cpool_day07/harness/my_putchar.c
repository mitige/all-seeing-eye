/*
** EPITECH PROJECT, 2026
** cpool_day07
** File description:
** my_putchar officiel de la moulinette
*/

#include <unistd.h>

void my_putchar(char c)
{
    write(1, &c, 1);
}
