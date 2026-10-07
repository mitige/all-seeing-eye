/*
** EPITECH PROJECT, 2026
** cpool_day06
** File description:
** main de test pour my_str_isprintable
*/

#include <stdio.h>

int my_str_isprintable(char const *str);

int main(void)
{
    printf("%d\n", my_str_isprintable("Hello ~!"));
    printf("%d\n", my_str_isprintable("hi\tthere"));
    printf("%d\n", my_str_isprintable(""));
    printf("%d\n", my_str_isprintable("a\nb"));
    printf("%d\n", my_str_isprintable("del\x7f" "ete"));
    printf("%d\n", my_str_isprintable("printable 42"));
    return (0);
}
