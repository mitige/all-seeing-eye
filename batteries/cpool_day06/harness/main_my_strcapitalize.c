/*
** EPITECH PROJECT, 2026
** cpool_day06
** File description:
** main de test pour my_strcapitalize
*/

#include <stdio.h>
#include <string.h>

char *my_strcapitalize(char *str);

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
    char buf[64];
    char *ret;

    memset(buf, '#', 63);
    buf[63] = '\0';
    strcpy(buf, src);
    ret = my_strcapitalize(buf);
    printf("ret=%s|buf=", ret);
    dump_buf(buf, 64);
    printf("\n");
}

int main(void)
{
    test("hey, how are you? 42WORds forty-two; fifty+one");
    test("");
    test("hello");
    test("a1b2c");
    test("tEST-uPPER");
    return (0);
}
