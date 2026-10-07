/*
** EPITECH PROJECT, 2026
** cpool_day08
** File description:
** my_strdup
*/

#include <stdlib.h>

static int my_len(char const *str)
{
    int len = 0;

    while (str[len] != '\0') {
        len = len + 1;
    }
    return (len);
}

char *my_strdup(char const *src)
{
    int len = my_len(src);
    char *dup = malloc(sizeof(char) * (len + 1));
    int i = 0;

    if (dup == NULL) {
        return (NULL);
    }
    while (i < len) {
        dup[i] = src[i];
        i = i + 1;
    }
    dup[len] = '\0';
    return (dup);
}
