/*
** EPITECH PROJECT, 2026
** cpool_day09
** File description:
** my_putstr de référence (libmy de la batterie)
*/

void my_putchar(char c);

int my_putstr(char const *str)
{
    int i = 0;

    while (str[i] != '\0') {
        my_putchar(str[i]);
        i++;
    }
    return (0);
}
