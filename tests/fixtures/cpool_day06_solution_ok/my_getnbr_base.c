/*
** EPITECH PROJECT, 2026
** cpool_day06
** File description:
** my_getnbr_base
*/

static int base_len(char const *base)
{
    int i = 0;

    while (base[i] != '\0')
        i++;
    return (i);
}

static int base_index(char const *base, char c)
{
    int i = 0;

    while (base[i] != '\0') {
        if (base[i] == c)
            return (i);
        i++;
    }
    return (-1);
}

static int base_valid(char const *base, int len)
{
    int i = 0;
    int j;

    if (len < 2)
        return (0);
    while (i < len) {
        j = i + 1;
        while (j < len && base[i] != base[j])
            j++;
        if (j < len)
            return (0);
        i++;
    }
    return (1);
}

static unsigned int parse_digits(char const *str, int i, char const *base)
{
    unsigned int n = 0;
    int len = base_len(base);

    while (base_index(base, str[i]) >= 0) {
        n = n * (unsigned int)len + (unsigned int)base_index(base, str[i]);
        i++;
    }
    return (n);
}

int my_getnbr_base(char const *str, char const *base)
{
    int i = 0;
    int sign = 1;
    unsigned int n;

    if (!base_valid(base, base_len(base)))
        return (0);
    while (str[i] == '-' || str[i] == '+') {
        if (str[i] == '-')
            sign = -sign;
        i++;
    }
    if (base_index(base, str[i]) < 0)
        return (0);
    n = parse_digits(str, i, base);
    if (sign < 0)
        return ((int)(0 - n));
    return ((int)n);
}
