/*
** EPITECH PROJECT, 2026
** cpool_day06
** File description:
** main de test pour my_strncpy
*/

#include <stdio.h>
#include <string.h>

char *my_strncpy(char *dest, char const *src, int n);

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

static void test(char const *src, int n)
{
    char dest[24];
    char *ret;

    memset(dest, '#', 23);
    dest[23] = '\0';
    ret = my_strncpy(dest, src, n);
    printf("ret=%s|buf=", ret);
    dump_buf(dest, 24);
    printf("\n");
}

int main(void)
{
    test("HelloWorld", 5);
    test("Hello", 6);
    test("Hello", 5);
    test("Hello", 0);
    return (0);
}
