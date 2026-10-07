/*
** EPITECH PROJECT, 2026
** cpool_day06
** File description:
** my_strlowcase
*/

char *my_strlowcase(char *str)
{
    int i = 0;

    while (str[i] != '\0') {
        if (str[i] >= 'A' && str[i] <= 'Z')
            str[i] = str[i] - 'A' + 'a';
        i++;
    }
    return (str);
}
