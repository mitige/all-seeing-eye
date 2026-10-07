/*
** EPITECH PROJECT, 2026
** cpool_day06
** File description:
** main de test pour my_str_isupper
*/

#include <stdio.h>

int my_str_isupper(char const *str);

int main(void)
{
    printf("%d\n", my_str_isupper("HELLO"));
    printf("%d\n", my_str_isupper("HellO"));
    printf("%d\n", my_str_isupper(""));
    printf("%d\n", my_str_isupper("ABCXYZ"));
    printf("%d\n", my_str_isupper("ABC1"));
    return (0);
}
