/*
** EPITECH PROJECT, 2026
** cpool_day07
** File description:
** my_strcapitalize
*/

static int is_alnum(char c)
{
    if (c >= '0' && c <= '9') {
        return (1);
    }
    if (c >= 'a' && c <= 'z') {
        return (1);
    }
    return (c >= 'A' && c <= 'Z');
}

char *my_strcapitalize(char *str)
{
    int i = 0;
    int debut;

    while (str[i] != '\0') {
        if (str[i] >= 'A' && str[i] <= 'Z') {
            str[i] = str[i] + ('a' - 'A');
        }
        debut = (i == 0 || !is_alnum(str[i - 1]));
        if (debut && str[i] >= 'a' && str[i] <= 'z') {
            str[i] = str[i] - ('a' - 'A');
        }
        i = i + 1;
    }
    return (str);
}
