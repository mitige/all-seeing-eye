/*
** EPITECH PROJECT, 2026
** cpool_day06
** File description:
** main de test pour my_str_islower
*/

#include <stdio.h>

int my_str_islower(char const *str);

int main(void)
{
    printf("%d\n", my_str_islower("hello"));
    printf("%d\n", my_str_islower("Hello"));
    printf("%d\n", my_str_islower(""));
    printf("%d\n", my_str_islower("abcxyz"));
    printf("%d\n", my_str_islower("abc1"));
    return (0);
}
