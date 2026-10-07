/*
** EPITECH PROJECT, 2026
** cpool_day04
** File description:
** main de test pour my_swap
*/

#include <stdio.h>

void my_swap(int *a, int *b);

int main(void)
{
    int a = 1;
    int b = 2;
    int c = -42;
    int d = 42;

    my_swap(&a, &b);
    printf("%d %d\n", a, b);
    my_swap(&c, &d);
    printf("%d %d\n", c, d);
    my_swap(&a, &a);
    printf("%d\n", a);
    return (0);
}
