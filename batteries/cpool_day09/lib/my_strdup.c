/*
** EPITECH PROJECT, 2026
** cpool_day09
** File description:
** my_strdup de référence (libmy de la batterie)
*/

#include <stdlib.h>

int my_strlen(char const *str);

char *my_strdup(char const *src)
{
    int len = my_strlen(src);
    char *copy = malloc(sizeof(char) * (len + 1));
    int i = 0;

    if (copy == NULL)
        return (NULL);
    while (i < len) {
        copy[i] = src[i];
        i++;
    }
    copy[len] = '\0';
    return (copy);
}
