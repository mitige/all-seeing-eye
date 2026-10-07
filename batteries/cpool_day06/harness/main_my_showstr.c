/*
** EPITECH PROJECT, 2026
** cpool_day06
** File description:
** main de test pour my_showstr
*/

#include <stdio.h>

int my_showstr(char const *str);

int main(void)
{
    setvbuf(stdout, NULL, _IONBF, 0);
    printf("|%d\n", my_showstr("I like \n ponies!"));
    printf("|%d\n", my_showstr(""));
    printf("|%d\n", my_showstr("Hello"));
    printf("|%d\n", my_showstr("tab\there"));
    printf("|%d\n", my_showstr("del\x7f" "ete"));
    printf("|%d\n", my_showstr("hi\xff" "ho"));
    printf("|%d\n", my_showstr("bell\aring"));
    return (0);
}
