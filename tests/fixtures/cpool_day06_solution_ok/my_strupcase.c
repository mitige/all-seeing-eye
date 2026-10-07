/*
** EPITECH PROJECT, 2026
** cpool_day06
** File description:
** my_strupcase
*/

char *my_strupcase(char *str)
{
    int i = 0;

    while (str[i] != '\0') {
        if (str[i] >= 'a' && str[i] <= 'z')
            str[i] = str[i] - 'a' + 'A';
        i++;
    }
    return (str);
}
