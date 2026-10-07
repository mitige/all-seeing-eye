/*
** EPITECH PROJECT, 2026
** cpool_workshoplib
** File description:
** my_strcmp
*/

int my_strcmp(char const *s1, char const *s2)
{
    int i = 0;

    while (s1[i] != '\0' && s1[i] == s2[i]) {
        i = i + 1;
    }
    return ((unsigned char)s1[i] - (unsigned char)s2[i]);
}
