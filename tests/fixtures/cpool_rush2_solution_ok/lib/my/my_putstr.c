/*
** EPITECH PROJECT, 2026
** cpool_day07
** File description:
** my_putstr
*/

#include <unistd.h>

int my_putstr(char const *str)
{
    int len = 0;

    while (str[len] != '\0') {
        len = len + 1;
    }
    write(1, str, len);
    return (0);
}
