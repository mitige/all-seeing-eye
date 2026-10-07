/*
** EPITECH PROJECT, 2026
** cpool_day10
** File description:
** my_advanced_do_op.c
*/

#include "../include/my.h"
#include "../include/my_opp.h"

t_opp const op_tab[] = {
    {"+", &my_add},
    {"-", &my_sub},
    {"/", &my_div},
    {"*", &my_mul},
    {"%", &my_mod},
    {"", &my_usage}
};

static int op_matches(char const *arg, char const *op)
{
    int i = 0;

    while (op[i] != '\0') {
        if (arg[i] != op[i])
            return (0);
        i++;
    }
    return (1);
}

static int find_op(char const *arg)
{
    int i = 0;

    while (op_tab[i].op[0] != '\0' && !op_matches(arg, op_tab[i].op))
        i++;
    return (i);
}

int main(int argc, char **argv)
{
    int i;
    int a;
    int b;

    if (argc != 4)
        return (84);
    i = find_op(argv[2]);
    if (op_tab[i].op[0] == '\0')
        return (op_tab[i].f(0, 0));
    a = my_getnbr(argv[1]);
    b = my_getnbr(argv[3]);
    if ((op_tab[i].f == &my_div || op_tab[i].f == &my_mod) && b == 0)
        return (op_tab[i].f(a, b));
    my_put_nbr(op_tab[i].f(a, b));
    my_putchar('\n');
    return (0);
}
