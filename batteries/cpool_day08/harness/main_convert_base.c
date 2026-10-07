/*
** EPITECH PROJECT, 2026
** cpool_day08
** File description:
** main de test pour convert_base
*/

#include <stdlib.h>
#include <string.h>
#include <unistd.h>

char *convert_base(char const *nbr, char const *from, char const *to);

static void show(char const *nbr, char const *from, char const *to)
{
    char *out = convert_base(nbr, from, to);

    if (out == NULL) {
        write(1, "NULL\n", 5);
        return;
    }
    write(1, out, strlen(out));
    write(1, "\n", 1);
}

int main(void)
{
    show("101011", "01", "0123456789");
    show("43", "0123456789", "01");
    show("-2a", "0123456789abcdef", "0123456789");
    show("2A", "0123456789ABCDEF", "0123456789");
    show("-2147483648", "0123456789", "01");
    show("0", "0123456789", "0123456789ABCDEF");
    show("+52", "01234567", "0123456789");
    show("-+52", "01234567", "0123456789");
    show("-poney", "poneyvif", "0123456789");
    return (0);
}
