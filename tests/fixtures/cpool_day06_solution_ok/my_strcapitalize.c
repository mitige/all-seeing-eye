/*
** EPITECH PROJECT, 2026
** cpool_day06
** File description:
** my_strcapitalize
*/

static int is_alnum(char c)
{
    if (c >= 'a' && c <= 'z')
        return (1);
    if (c >= 'A' && c <= 'Z')
        return (1);
    if (c >= '0' && c <= '9')
        return (1);
    return (0);
}

char *my_strcapitalize(char *str)
{
    int i = 0;

    while (str[i] != '\0') {
        if (str[i] >= 'A' && str[i] <= 'Z')
            str[i] = str[i] - 'A' + 'a';
        if ((i == 0 || !is_alnum(str[i - 1]))
            && str[i] >= 'a' && str[i] <= 'z')
            str[i] = str[i] - 'a' + 'A';
        i++;
    }
    return (str);
}
