/*
** EPITECH PROJECT, 2026
** cpool_day06
** File description:
** main de test pour my_str_isalpha
*/

#include <stdio.h>

int my_str_isalpha(char const *str);

int main(void)
{
    printf("%d\n", my_str_isalpha("Hello"));
    printf("%d\n", my_str_isalpha("Hello42"));
    printf("%d\n", my_str_isalpha(""));
    printf("%d\n", my_str_isalpha("abcXYZ"));
    printf("%d\n", my_str_isalpha("hi there"));
    return (0);
}
