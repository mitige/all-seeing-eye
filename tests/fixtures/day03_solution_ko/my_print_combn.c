/*
** EPITECH PROJECT, 2026
** cpool_day03
** File description:
** my_print_combn
*/

void my_putchar(char c);

static void print_padded(unsigned int value, int width)
{
    if (width > 1) {
        print_padded(value / 10, width - 1);
    }
    my_putchar('0' + (int)(value % 10));
}

static int first_digit(unsigned int value, int width)
{
    if (width > 1) {
        return (first_digit(value / 10, width - 1));
    }
    return ((int)(value % 10));
}

static void print_separator(int first, int n)
{
    if (first != 10 - n) {
        my_putchar(',');
        my_putchar(' ');
    }
}

static void combn_rec(int n, int start, int depth, unsigned int acc)
{
    int d;

    d = start;
    while (d <= 10 - n + depth) {
        if (depth + 1 == n) {
            print_padded(acc * 10 + (unsigned int)d, n);
            print_separator(first_digit(acc * 10 + (unsigned int)d, n), n);
        } else {
            combn_rec(n, d + 1, depth + 1, acc * 10 + (unsigned int)d);
        }
        d = d + 1;
    }
}

int my_print_combn(int n)
{
    if (n <= 0 || n >= 10) {
        return (0);
    }
    combn_rec(n, 0, 0, 0);
    return (0);
}
