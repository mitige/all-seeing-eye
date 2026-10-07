/*
** EPITECH PROJECT, 2026
** cpool_countisland
** File description:
** my_getnbr
*/

static int overflows(long nbr, int sign)
{
    int over;

    over = (sign == 1 && nbr > 2147483647);
    over = over || (sign == -1 && nbr > 2147483648L);
    return (over);
}

int my_getnbr(char const *str)
{
    long int nbr = 0;
    int sign = 1;
    int i = 0;

    while (str[i] == '+' || str[i] == '-') {
        if (str[i] == '-') {
            sign = -sign;
        }
        i = i + 1;
    }
    while (str[i] >= '0' && str[i] <= '9') {
        nbr = nbr * 10 + (str[i] - '0');
        if (overflows(nbr, sign)) {
            return (0);
        }
        i = i + 1;
    }
    return ((int)(nbr * sign));
}
