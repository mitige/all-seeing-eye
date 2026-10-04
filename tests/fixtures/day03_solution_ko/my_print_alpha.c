/*
** EPITECH PROJECT, 2026
** cpool_day03
** File description:
** my_print_alpha
*/

void my_putchar(char c);

int my_print_alpha(void)
{
    char c;

    c = 'a';
    while (c < 'z') {
        my_putchar(c);
        c = c + 1;
    }
    return (0);
}
