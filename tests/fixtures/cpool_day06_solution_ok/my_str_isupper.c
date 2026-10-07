/*
** EPITECH PROJECT, 2026
** cpool_day06
** File description:
** my_str_isupper
*/

int my_str_isupper(char const *str)
{
    int i = 0;

    while (str[i] != '\0') {
        if (str[i] < 'A' || str[i] > 'Z')
            return (0);
        i++;
    }
    return (1);
}
