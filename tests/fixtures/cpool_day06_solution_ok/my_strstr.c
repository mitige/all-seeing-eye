/*
** EPITECH PROJECT, 2026
** cpool_day06
** File description:
** my_strstr
*/

char *my_strstr(char *str, char const *to_find)
{
    int i = 0;
    int j;

    if (to_find[0] == '\0')
        return (str);
    while (str[i] != '\0') {
        j = 0;
        while (to_find[j] != '\0' && str[i + j] == to_find[j])
            j++;
        if (to_find[j] == '\0')
            return (str + i);
        i++;
    }
    return (0);
}
