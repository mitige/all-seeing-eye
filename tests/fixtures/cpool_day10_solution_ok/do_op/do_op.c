/*
** EPITECH PROJECT, 2026
** cpool_day10
** File description:
** do_op.c
*/

#include <unistd.h>
#include "../include/my.h"

struct s_op {
    char op;
    int (*f)(int, int);
};

static void put_err(char const *str)
{
    write(2, str, my_strlen(str));
}

static int op_add(int a, int b)
{
    return (a + b);
}

static int op_sub(int a, int b)
{
    return (a - b);
}

static int op_mul(int a, int b)
{
    return (a * b);
}

static int op_div(int a, int b)
{
    if (b == 0) {
        put_err("Stop: division by zero\n");
        return (84);
    }
    return (a / b);
}

static int op_mod(int a, int b)
{
    if (b == 0) {
        put_err("Stop: modulo by zero\n");
        return (84);
    }
    return (a % b);
}
static struct s_op const OPS[] = {
    {'+', &op_add},
    {'-', &op_sub},
    {'*', &op_mul},
    {'/', &op_div},
    {'%', &op_mod}
};

static int find_op(char c)
{
    int i = 0;

    while (i < 5) {
        if (OPS[i].op == c)
            return (i);
        i++;
    }
    return (-1);
}

int main(int argc, char **argv)
{
    int i;
    int a;
    int b;

    if (argc != 4)
        return (84);
    i = find_op(argv[2][0]);
    if (i == -1) {
        my_putchar('0');
        my_putchar('\n');
        return (84);
    }
    a = my_getnbr(argv[1]);
    b = my_getnbr(argv[3]);
    if ((OPS[i].f == &op_div || OPS[i].f == &op_mod) && b == 0)
        return (OPS[i].f(a, b));
    my_put_nbr(OPS[i].f(a, b));
    my_putchar('\n');
    return (0);
}
