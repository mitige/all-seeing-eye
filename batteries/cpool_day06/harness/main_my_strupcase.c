/*
** EPITECH PROJECT, 2026
** cpool_day06
** File description:
** main de test pour my_strupcase
*/

#include <stdio.h>
#include <string.h>

char *my_strupcase(char *str);

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
    char buf[24];
    char *ret;

    memset(buf, '#', 23);
    buf[23] = '\0';
    strcpy(buf, src);
    ret = my_strupcase(buf);
    printf("ret=%s|buf=", ret);
    dump_buf(buf, 24);
    printf("\n");
}

int main(void)
{
    test("hello");
    test("Hello World 42!");
    test("");
    test("42 already!");
    return (0);
}
