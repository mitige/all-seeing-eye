/*
** EPITECH PROJECT, 2026
** cpool_day06
** File description:
** my_showstr
*/

void my_putchar(char c);

static void put_hex(unsigned char c)
{
    char const *hex = "0123456789abcdef";

    my_putchar('\\');
    my_putchar(hex[c / 16]);
    my_putchar(hex[c % 16]);
}

int my_showstr(char const *str)
{
    int i = 0;

    while (str[i] != '\0') {
        if (str[i] >= 32 && str[i] <= 126)
            my_putchar(str[i]);
        else
            put_hex((unsigned char)str[i]);
        i++;
    }
    return (0);
}
