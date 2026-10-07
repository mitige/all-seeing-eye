/*
** EPITECH PROJECT, 2026
** cpool_day10
** File description:
** my_mod.c
*/

#include <unistd.h>
#include "../include/my.h"

int my_mod(int a, int b)
{
    if (b == 0) {
        write(2, "Stop: modulo by zero\n", 21);
        return (84);
    }
    return (a % b);
}
