/*
** EPITECH PROJECT, 2026
** cpool_day06
** File description:
** my_revstr
*/

char *my_revstr(char *str)
{
    int len = 0;
    int i = 0;
    char tmp;

    while (str[len] != '\0')
        len++;
    while (i < len / 2) {
        tmp = str[i];
        str[i] = str[len - 1 - i];
        str[len - 1 - i] = tmp;
        i++;
    }
    return (str);
}
