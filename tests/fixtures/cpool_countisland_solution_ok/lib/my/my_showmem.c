/*
** EPITECH PROJECT, 2026
** cpool_countisland
** File description:
** my_showmem
*/

#include <unistd.h>

static void put_c(char c)
{
    write(1, &c, 1);
}

static void print_hex(unsigned char c)
{
    const char *hex = "0123456789abcdef";

    put_c(hex[c / 16]);
    put_c(hex[c % 16]);
}

static void print_addr(int addr)
{
    int shift = 28;

    while (shift >= 0) {
        put_c("0123456789abcdef"[(unsigned int)addr >> shift & 0xf]);
        shift = shift - 4;
    }
}

static void print_line(char const *str, int start, int size)
{
    int i = 0;

    print_addr(start);
    put_c(':');
    put_c(' ');
    while (i < 16) {
        if (start + i < size) {
            print_hex((unsigned char)str[start + i]);
        } else {
            put_c(' ');
            put_c(' ');
        }
        if (i % 2 == 1) {
            put_c(' ');
        }
        i = i + 1;
    }
}

int my_showmem(char const *str, int size)
{
    int start = 0;

    while (start < size) {
        print_line(str, start, size);
        put_c('\n');
        start = start + 16;
    }
    return (0);
}
