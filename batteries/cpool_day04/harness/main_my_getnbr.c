/*
** EPITECH PROJECT, 2026
** cpool_day04
** File description:
** main de test pour my_getnbr
*/

#include <stdio.h>

int my_getnbr(char const *str);

int main(void)
{
    printf("%d\n", my_getnbr("42"));
    printf("%d\n", my_getnbr("-42"));
    printf("%d\n", my_getnbr("+42"));
    printf("%d\n", my_getnbr("--42-42"));
    printf("%d\n", my_getnbr("+---+--++---+---+---+-42"));
    printf("%d\n", my_getnbr("42a43"));
    printf("%d\n", my_getnbr("11000000000000000000000042"));
    printf("%d\n", my_getnbr("-1000000000000000000000042"));
    printf("%d\n", my_getnbr("2147483647"));
    printf("%d\n", my_getnbr("-2147483648"));
    printf("%d\n", my_getnbr("2147483648"));
    printf("%d\n", my_getnbr("-2147483649"));
    printf("%d\n", my_getnbr(""));
    printf("%d\n", my_getnbr("0"));
    printf("%d\n", my_getnbr("-0"));
    return (0);
}
