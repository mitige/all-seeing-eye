/*
** EPITECH PROJECT, 2026
** cpool_day10
** File description:
** my_div.c
*/

#include <unistd.h>
#include "../include/my.h"

int my_div(int a, int b)
{
    if (b == 0) {
        write(2, "Stop: division by zero\n", 23);
        return (84);
    }
    return (a / b);
}
