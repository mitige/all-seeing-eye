/*
** EPITECH PROJECT, 2026
** cpool_day04
** File description:
** main de test pour my_strlen
*/

#include <stdio.h>

int my_strlen(char const *str);

int main(void)
{
    printf("%d\n", my_strlen(""));
    printf("%d\n", my_strlen("a"));
    printf("%d\n", my_strlen("Hello World"));
    printf("%d\n", my_strlen("0123456789"));
    return (0);
}
