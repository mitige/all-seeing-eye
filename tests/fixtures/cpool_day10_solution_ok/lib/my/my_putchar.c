/*
** EPITECH PROJECT, 2026
** cpool_day10
** File description:
** my_putchar.c
*/

#include <unistd.h>

void my_putchar(char c)
{
    write(1, &c, 1);
}
