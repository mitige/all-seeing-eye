/*
** EPITECH PROJECT, 2026
** cpool_day06
** File description:
** main de test pour my_strcmp
*/

#include <stdio.h>

int my_strcmp(char const *s1, char const *s2);

int main(void)
{
    printf("%d\n", my_strcmp("abc", "abc"));
    printf("%d\n", my_strcmp("abc", "abd"));
    printf("%d\n", my_strcmp("abd", "abc"));
    printf("%d\n", my_strcmp("abc", "abcd"));
    printf("%d\n", my_strcmp("abcd", "abc"));
    printf("%d\n", my_strcmp("", ""));
    printf("%d\n", my_strcmp("", "a"));
    printf("%d\n", my_strcmp("a", ""));
    printf("%d\n", my_strcmp("Hello", "Hello"));
    return (0);
}
