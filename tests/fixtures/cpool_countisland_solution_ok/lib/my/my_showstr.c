/*
** EPITECH PROJECT, 2026
** cpool_countisland
** File description:
** my_showstr
*/

#include <unistd.h>

static void put_c(char c)
{
    write(1, &c, 1);
}

static void print_hex(unsigned char c)
{
    char const *hex = "0123456789abcdef";

    put_c('\\');
    put_c(hex[c / 16]);
    put_c(hex[c % 16]);
}

int my_showstr(char const *str)
{
    int i = 0;

    while (str[i] != '\0') {
        if (str[i] >= 32 && str[i] <= 126) {
            put_c(str[i]);
        } else {
            print_hex((unsigned char)str[i]);
        }
        i = i + 1;
    }
    return (0);
}
