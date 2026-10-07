/*
** EPITECH PROJECT, 2026
** cpool_countisland
** File description:
** my_strncmp
*/

int my_strncmp(char const *s1, char const *s2, int n)
{
    int i = 0;

    if (n <= 0) {
        return (0);
    }
    while (i < n - 1 && s1[i] != '\0' && s1[i] == s2[i]) {
        i = i + 1;
    }
    return ((unsigned char)s1[i] - (unsigned char)s2[i]);
}
