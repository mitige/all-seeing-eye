/*
** EPITECH PROJECT, 2026
** cpool_day03
** File description:
** my_isneg
*/

void my_putchar(char c);

int my_isneg(int n)
{
    if (n < 0) {
        my_putchar('N');
    } else {
        my_putchar('P');
    }
    return (0);
}
