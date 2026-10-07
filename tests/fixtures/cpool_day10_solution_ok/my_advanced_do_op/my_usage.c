/*
** EPITECH PROJECT, 2026
** cpool_day10
** File description:
** my_usage.c
*/

#include <unistd.h>
#include "../include/my.h"
#include "../include/my_opp.h"

extern t_opp const op_tab[];

static void put_err(char const *str)
{
    write(2, str, my_strlen(str));
}

int my_usage(int a, int b)
{
    int i = 0;

    (void)a;
    (void)b;
    put_err("error: only [ ");
    while (op_tab[i].op[0] != '\0') {
        put_err(op_tab[i].op);
        put_err(" ");
        i++;
    }
    put_err("] are supported\n");
    return (84);
}
