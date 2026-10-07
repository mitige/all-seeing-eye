/*
** EPITECH PROJECT, 2026
** cpool_countisland
** File description:
** my_strncpy
*/

char *my_strncpy(char *dest, char const *src, int n)
{
    int i = 0;

    while (i < n && src[i] != '\0') {
        dest[i] = src[i];
        i = i + 1;
    }
    while (i < n) {
        dest[i] = '\0';
        i = i + 1;
    }
    return (dest);
}
