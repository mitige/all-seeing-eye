/*
** EPITECH PROJECT, 2026
** cpool_day06
** File description:
** main de test pour my_strstr
*/

#include <stdio.h>

char *my_strstr(char *str, char const *to_find);

static void test(char *str, char const *to_find)
{
    char *ret = my_strstr(str, to_find);

    printf("ret=%s\n", ret ? ret : "(null)");
}

int main(void)
{
    char h1[] = "Hello World";
    char h2[] = "Hello World";
    char h3[] = "aaa";
    char h4[] = "abc";
    char h5[] = "";
    char h6[] = "abc";
    char h7[] = "aaaaab";
    char h8[] = "abc";

    test(h1, "World");
    test(h2, "o W");
    test(h3, "aa");
    test(h4, "");
    test(h5, "a");
    test(h6, "d");
    test(h7, "aaab");
    test(h8, "abcd");
    return (0);
}
