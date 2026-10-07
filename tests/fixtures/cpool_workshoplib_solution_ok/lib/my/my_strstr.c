/*
** EPITECH PROJECT, 2026
** cpool_workshoplib
** File description:
** my_strstr
*/

char *my_strstr(char *str, char const *to_find)
{
    int i = 0;
    int j;

    if (to_find[0] == '\0') {
        return (str);
    }
    while (str[i] != '\0') {
        j = 0;
        while (to_find[j] != '\0' && str[i + j] == to_find[j]) {
            j = j + 1;
        }
        if (to_find[j] == '\0') {
            return (str + i);
        }
        i = i + 1;
    }
    return ((char *)0);
}
