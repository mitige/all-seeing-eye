/*
** EPITECH PROJECT, 2026
** cpool_day08
** File description:
** convert_base
*/

#include <stdlib.h>

static int base_index(char const *base, char c)
{
    int i = 0;

    while (base[i] != '\0') {
        if (base[i] == c) {
            return (i);
        }
        i = i + 1;
    }
    return (-1);
}

static int base_length(char const *base)
{
    int len = 0;

    while (base[len] != '\0') {
        len = len + 1;
    }
    return (len);
}

static unsigned int parse_base(char const *nbr, char const *base, int *neg)
{
    unsigned int value = 0;
    int radix = base_length(base);
    int i = 0;
    int digit = 0;

    *neg = 0;
    while (nbr[i] == '-' || nbr[i] == '+') {
        if (nbr[i] == '-') {
            *neg = !(*neg);
        }
        i = i + 1;
    }
    digit = base_index(base, nbr[i]);
    while (digit >= 0) {
        value = value * (unsigned int)radix + (unsigned int)digit;
        i = i + 1;
        digit = base_index(base, nbr[i]);
    }
    return (value);
}

static int count_digits(unsigned int value, int radix)
{
    int count = 1;

    while (value >= (unsigned int)radix) {
        value = value / (unsigned int)radix;
        count = count + 1;
    }
    return (count);
}

char *convert_base(char const *nbr, char const *base_from, char const *base_to)
{
    int neg = 0;
    unsigned int value = parse_base(nbr, base_from, &neg);
    int radix = base_length(base_to);
    int digits = count_digits(value, radix);
    char *out = malloc(sizeof(char) * (digits + neg + 1));
    int i = digits + neg;

    if (out == NULL) {
        return (NULL);
    }
    out[i] = '\0';
    while (i > neg) {
        i = i - 1;
        out[i] = base_to[value % (unsigned int)radix];
        value = value / (unsigned int)radix;
    }
    if (neg) {
        out[0] = '-';
    }
    return (out);
}
