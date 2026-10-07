/*
** EPITECH PROJECT, 2026
** cpool_day04
** File description:
** my_getnbr
*/

int my_getnbr(char const *str)
{
    long nb = 0;
    int sign = 1;
    int i = 0;

    while (str[i] == '-' || str[i] == '+') {
        if (str[i] == '-')
            sign = -sign;
        i = i + 1;
    }
    while (str[i] >= '0' && str[i] <= '9') {
        nb = nb * 10 + str[i] - '0';
        if (nb > 2147483648L)
            return (0);
        i = i + 1;
    }
    nb = nb * sign;
    if (nb > 2147483647L || nb < -2147483648L)
        return (0);
    return ((int)nb);
}
