/*
** EPITECH PROJECT, 2026
** cpool_day06
** File description:
** main de test pour my_strncmp
*/

#include <stdio.h>

int my_strncmp(char const *s1, char const *s2, int n);

int main(void)
{
    printf("%d\n", my_strncmp("abcdef", "abcXYZ", 3));
    printf("%d\n", my_strncmp("abcdef", "abd", 3));
    printf("%d\n", my_strncmp("abc", "abcdef", 10));
    printf("%d\n", my_strncmp("a", "b", 0));
    printf("%d\n", my_strncmp("abc", "abc", 0));
    printf("%d\n", my_strncmp("abc", "abd", 2));
    printf("%d\n", my_strncmp("abc", "abd", 3));
    printf("%d\n", my_strncmp("", "", 5));
    printf("%d\n", my_strncmp("", "a", 1));
    return (0);
}
