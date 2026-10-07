/*
** EPITECH PROJECT, 2026
** cpool_day04
** File description:
** my_putstr
*/

void my_putchar(char c);

int my_putstr(char const *str)
{
    int i;

    i = 0;
    while (str[i] != '\0') {
        my_putchar(str[i]);
        i = i + 1;
    }
    return (0);
}
