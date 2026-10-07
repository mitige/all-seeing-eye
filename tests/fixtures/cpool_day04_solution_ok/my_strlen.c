/*
** EPITECH PROJECT, 2026
** cpool_day04
** File description:
** my_strlen
*/

int my_strlen(char const *str)
{
    int len;

    len = 0;
    while (str[len] != '\0')
        len = len + 1;
    return (len);
}
