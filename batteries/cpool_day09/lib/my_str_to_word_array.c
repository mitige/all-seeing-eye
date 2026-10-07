/*
** EPITECH PROJECT, 2026
** cpool_day09
** File description:
** my_str_to_word_array de référence (libmy de la batterie)
** Séparateurs : espace et tabulation (Day08 canonique).
*/

#include <stdlib.h>

static int is_sep(char c)
{
    return (c == ' ' || c == '\t');
}

static int count_words(char const *str)
{
    int count = 0;
    int i = 0;

    while (str[i] != '\0') {
        if (!is_sep(str[i]) && (i == 0 || is_sep(str[i - 1])))
            count++;
        i++;
    }
    return (count);
}

static int word_end(char const *str, int i)
{
    while (str[i] != '\0' && !is_sep(str[i]))
        i++;
    return (i);
}

static char *extract_word(char const *str, int start, int len)
{
    char *word = malloc(sizeof(char) * (len + 1));
    int i = 0;

    if (word == NULL)
        return (NULL);
    while (i < len) {
        word[i] = str[start + i];
        i++;
    }
    word[len] = '\0';
    return (word);
}

char **my_str_to_word_array(char const *str)
{
    char **array = malloc(sizeof(char *) * (count_words(str) + 1));
    int i = 0;
    int w = 0;
    int start = 0;

    if (array == NULL)
        return (NULL);
    while (str[i] != '\0') {
        if (!is_sep(str[i]) && (i == 0 || is_sep(str[i - 1]))) {
            start = i;
            i = word_end(str, i);
            array[w] = extract_word(str, start, i - start);
            w++;
        } else {
            i++;
        }
    }
    array[w] = NULL;
    return (array);
}
