/*
** EPITECH PROJECT, 2026
** cpool_day07
** File description:
** my_revstr
*/

char *my_revstr(char *str)
{
    int i = 0;
    int j = 0;
    char tmp;

    while (str[j] != '\0') {
        j = j + 1;
    }
    j = j - 1;
    while (i < j) {
        tmp = str[i];
        str[i] = str[j];
        str[j] = tmp;
        i = i + 1;
        j = j - 1;
    }
    return (str);
}
