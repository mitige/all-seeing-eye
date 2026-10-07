/*
** EPITECH PROJECT, 2026
** cpool_day04
** File description:
** my_evil_str
*/

char *my_evil_str(char *str)
{
    int len;
    int i;
    char tmp;

    len = 0;
    while (str[len] != '\0')
        len = len + 1;
    i = 0;
    while (i < len / 2) {
        tmp = str[i];
        str[i] = str[len - 1 - i];
        str[len - 1 - i] = tmp;
        i = i + 1;
    }
    return (str);
}
