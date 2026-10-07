/*
** EPITECH PROJECT, 2026
** cpool_day09
** File description:
** my_putchar de référence (libmy de la batterie)
*/

#include <unistd.h>

void my_putchar(char c)
{
    write(1, &c, 1);
}
