/*
** EPITECH PROJECT, 2026
** cpool_day04
** File description:
** main de test pour my_evil_str
*/

#include <stdio.h>

char *my_evil_str(char *str);

int main(void)
{
    char s1[] = "a";
    char s2[] = "ab";
    char s3[] = "abc";
    char s4[] = "abcd";
    char s5[] = "abcde";
    char s6[] = "abcdef";
    char s7[] = "";
    char s8[] = "Hello World!";

    printf("%s\n", my_evil_str(s1));
    printf("%s\n", my_evil_str(s2));
    printf("%s\n", my_evil_str(s3));
    printf("%s\n", my_evil_str(s4));
    printf("%s\n", my_evil_str(s5));
    printf("%s\n", my_evil_str(s6));
    printf("%s\n", my_evil_str(s7));
    printf("%s\n", my_evil_str(s8));
    return (0);
}
