/*
** EPITECH PROJECT, 2026
** cpool_day03
** File description:
** my_print_comb
*/

void my_putchar(char c);

static void print_comb(char a, char b, char c)
{
    my_putchar(a);
    my_putchar(b);
    my_putchar(c);
    if (a != '7') {
        my_putchar(',');
        my_putchar(' ');
    }
}

static void print_from(char a, char b)
{
    char c;

    c = b + 1;
    while (c <= '9') {
        print_comb(a, b, c);
        c = c + 1;
    }
}

int my_print_comb(void)
{
    char a;
    char b;

    a = '0';
    while (a <= '7') {
        b = a + 1;
        while (b <= '8') {
            print_from(a, b);
            b = b + 1;
        }
        a = a + 1;
    }
    return (0);
}
