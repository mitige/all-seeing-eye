/*
** EPITECH PROJECT, 2026
** cpool_day08
** File description:
** my_str_to_word_array
*/

#include <stdlib.h>

static int is_alnum(char c)
{
    if (c >= 'a' && c <= 'z') {
        return (1);
    }
    if (c >= 'A' && c <= 'Z') {
        return (1);
    }
    if (c >= '0' && c <= '9') {
        return (1);
    }
    return (0);
}

static int count_words(char const *str)
{
    int count = 0;
    int i = 0;

    while (str[i] != '\0') {
        if (is_alnum(str[i]) && (i == 0 || !is_alnum(str[i - 1]))) {
            count = count + 1;
        }
        i = i + 1;
    }
    return (count);
}

static int word_length(char const *str)
{
    int len = 0;

    while (is_alnum(str[len])) {
        len = len + 1;
    }
    return (len);
}

static char *copy_word(char const *str, int len)
{
    char *word = malloc(sizeof(char) * (len + 1));
    int i = 0;

    if (word == NULL) {
        return (NULL);
    }
    while (i < len) {
        word[i] = str[i];
        i = i + 1;
    }
    word[len] = '\0';
    return (word);
}

char **my_str_to_word_array(char const *str)
{
    int words = count_words(str);
    char **tab = malloc(sizeof(char *) * (words + 1));
    int i = 0;
    int w = 0;

    if (tab == NULL) {
        return (NULL);
    }
    while (str[i] != '\0' && w < words) {
        if (is_alnum(str[i]) && (i == 0 || !is_alnum(str[i - 1]))) {
            tab[w] = copy_word(str + i, word_length(str + i));
            w = w + 1;
        }
        i = i + 1;
    }
    tab[w] = NULL;
    return (tab);
}
