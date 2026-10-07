/*
** EPITECH PROJECT, 2026
** cpool_day06
** File description:
** main de test pour my_getnbr_base
*/

#include <stdio.h>

int my_getnbr_base(char const *str, char const *base);

int main(void)
{
    printf("%d\n", my_getnbr_base("42", "0123456789"));
    printf("%d\n", my_getnbr_base("-42", "0123456789"));
    printf("%d\n", my_getnbr_base("---42", "0123456789"));
    printf("%d\n", my_getnbr_base("+-+42", "0123456789"));
    printf("%d\n", my_getnbr_base("101010", "01"));
    printf("%d\n", my_getnbr_base("2a", "0123456789abcdef"));
    printf("%d\n", my_getnbr_base("-2A", "0123456789ABCDEF"));
    printf("%d\n", my_getnbr_base("52", "01234567"));
    printf("%d\n", my_getnbr_base("", "0123456789"));
    printf("%d\n", my_getnbr_base("z", "0123456789"));
    printf("%d\n", my_getnbr_base("42", "0"));
    printf("%d\n", my_getnbr_base("42", "001"));
    printf("%d\n", my_getnbr_base("1111111111111111", "01"));
    printf("%d\n", my_getnbr_base("7fffffff", "0123456789abcdef"));
    printf("%d\n", my_getnbr_base("2147483647", "0123456789"));
    printf("%d\n", my_getnbr_base("-2147483648", "0123456789"));
    return (0);
}
