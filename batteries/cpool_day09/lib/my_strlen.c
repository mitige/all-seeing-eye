/*
** EPITECH PROJECT, 2026
** cpool_day09
** File description:
** my_strlen de référence (libmy de la batterie)
*/

int my_strlen(char const *str)
{
    int i = 0;

    while (str[i] != '\0')
        i++;
    return (i);
}
