/*
** EPITECH PROJECT, 2026
** cpool_day06
** File description:
** my_str_isalpha
*/

static int is_alpha(char c)
{
    if (c >= 'a' && c <= 'z')
        return (1);
    if (c >= 'A' && c <= 'Z')
        return (1);
    return (0);
}

int my_str_isalpha(char const *str)
{
    int i = 0;

    while (str[i] != '\0') {
        if (!is_alpha(str[i]))
            return (0);
        i++;
    }
    return (1);
}
