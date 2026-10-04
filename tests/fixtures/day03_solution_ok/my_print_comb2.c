/*
** EPITECH PROJECT, 2026
** cpool_day03
** File description:
** my_print_comb2
*/

void my_putchar(char c);

static void print_number(int n)
{
    my_putchar('0' + n / 10);
    my_putchar('0' + n % 10);
}

static void print_pair(int a, int b)
{
    print_number(a);
    my_putchar(' ');
    print_number(b);
    if (a != 98) {
        my_putchar(',');
        my_putchar(' ');
    }
}

int my_print_comb2(void)
{
    int a;
    int b;

    a = 0;
    while (a <= 98) {
        b = a + 1;
        while (b <= 99) {
            print_pair(a, b);
            b = b + 1;
        }
        a = a + 1;
    }
    return (0);
}
