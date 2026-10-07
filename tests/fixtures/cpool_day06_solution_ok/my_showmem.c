/*
** EPITECH PROJECT, 2026
** cpool_day06
** File description:
** my_showmem
*/

void my_putchar(char c);

static void put_hexn(unsigned int n, int width)
{
    char const *hex = "0123456789abcdef";

    if (width > 1)
        put_hexn(n / 16, width - 1);
    my_putchar(hex[n % 16]);
}

static void print_hex_area(char const *str, int start, int len)
{
    int j = 0;

    while (j < 16) {
        if (j < len)
            put_hexn((unsigned char)str[start + j], 2);
        else {
            my_putchar(' ');
            my_putchar(' ');
        }
        if (j % 2 == 1)
            my_putchar(' ');
        j++;
    }
    j = len;
    while (j < 16) {
        my_putchar(' ');
        j++;
    }
}

static void print_ascii(char const *str, int start, int len)
{
    int j = 0;

    while (j < len) {
        if (str[start + j] >= 32 && str[start + j] <= 126)
            my_putchar(str[start + j]);
        else
            my_putchar('.');
        j++;
    }
}

int my_showmem(char const *str, int size)
{
    int i = 0;
    int len;

    while (i < size) {
        len = size - i;
        if (len > 16)
            len = 16;
        put_hexn((unsigned int)i, 8);
        my_putchar(':');
        my_putchar(' ');
        print_hex_area(str, i, len);
        print_ascii(str, i, len);
        my_putchar('\n');
        i += 16;
    }
    return (0);
}
