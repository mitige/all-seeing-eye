/*
** EPITECH PROJECT, 2026
** cpool_day06
** File description:
** main de test pour my_str_isnum
*/

#include <stdio.h>

int my_str_isnum(char const *str);

int main(void)
{
    printf("%d\n", my_str_isnum("12345"));
    printf("%d\n", my_str_isnum("12a45"));
    printf("%d\n", my_str_isnum(""));
    printf("%d\n", my_str_isnum("0"));
    printf("%d\n", my_str_isnum("12 3"));
    return (0);
}
