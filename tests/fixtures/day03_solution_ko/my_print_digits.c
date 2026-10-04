/*
** EPITECH PROJECT, 2026
** cpool_day03
** File description:
** my_print_digits
*/

void my_putchar(char c);

int my_print_digits(void)
{
    char c;

    c = '0';
    while (c <= '9') {
        my_putchar(c);
        c = c + 1;
    }
    return (0);
}
