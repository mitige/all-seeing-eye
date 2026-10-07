/*
** EPITECH PROJECT, 2026
** cpool_day10
** File description:
** my_putstr.c
*/

#include <unistd.h>
#include "my.h"

int my_putstr(char const *str)
{
    write(1, str, my_strlen(str));
    return (0);
}
