/*
** EPITECH PROJECT, 2026
** cpool_day10
** File description:
** my_getnbr.c
*/

static int read_sign(char const *str, int *pos)
{
    int sign = 1;
    int i = 0;

    while (str[i] == '+' || str[i] == '-') {
        if (str[i] == '-')
            sign = -sign;
        i++;
    }
    *pos = i;
    return (sign);
}

int my_getnbr(char const *str)
{
    int i = 0;
    int sign = read_sign(str, &i);
    long n = 0;

    while (str[i] >= '0' && str[i] <= '9') {
        n = n * 10 + (str[i] - '0');
        if (n * sign > 2147483647 || n * sign < -2147483648L)
            return (0);
        i++;
    }
    return ((int)(n * sign));
}
