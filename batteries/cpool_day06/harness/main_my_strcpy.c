/*
** EPITECH PROJECT, 2026
** cpool_day06
** File description:
** main de test pour my_strcpy
*/

#include <stdio.h>
#include <string.h>

char *my_strcpy(char *dest, char const *src);

static void dump_byte(unsigned char c)
{
    if (c == 0) {
        printf("\\0");
        return;
    }
    if (c == '\\') {
        printf("\\\\");
        return;
    }
    if (c >= 32 && c <= 126) {
        putchar(c);
        return;
    }
    printf("\\x%02x", c);
}

static void dump_buf(char const *buf, int len)
{
    int i = 0;

    while (i < len) {
        dump_byte((unsigned char)buf[i]);
        i++;
    }
}

static void test(char const *src)
{
    char dest[24];
    char *ret;

    memset(dest, '#', 23);
    dest[23] = '\0';
    ret = my_strcpy(dest, src);
    printf("ret=%s|buf=", ret);
    dump_buf(dest, 24);
    printf("\n");
}

int main(void)
{
    test("Hello");
    test("");
    test("World of ponies 42");
    return (0);
}
