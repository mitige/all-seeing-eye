/*
** EPITECH PROJECT, 2026
** cpool_workshoplib
** File description:
** my_strncat
*/

char *my_strncat(char *dest, char const *src, int nb)
{
    int i = 0;
    int j = 0;

    while (dest[i] != '\0') {
        i = i + 1;
    }
    while (src[j] != '\0' && j < nb) {
        dest[i + j] = src[j];
        j = j + 1;
    }
    dest[i + j] = '\0';
    return (dest);
}
