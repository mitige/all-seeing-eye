/*
** EPITECH PROJECT, 2026
** cpool_rush2
** File description:
** letter counting and frequency display
*/

#include "my.h"
#include "rush2.h"

int is_letter(char c)
{
    return ((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z'));
}

char to_lower(char c)
{
    if (c >= 'A' && c <= 'Z')
        return (c + 'a' - 'A');
    return (c);
}

int count_letter(char const *text, char letter)
{
    int total = 0;
    int i = 0;

    while (text[i] != '\0') {
        if (is_letter(text[i]) && to_lower(text[i]) == to_lower(letter))
            total = total + 1;
        i = i + 1;
    }
    return (total);
}

int count_all_letters(char const *text)
{
    int total = 0;
    int i = 0;

    while (text[i] != '\0') {
        if (is_letter(text[i]))
            total = total + 1;
        i = i + 1;
    }
    return (total);
}

static void display_letter_line(char letter, int count, int total)
{
    int centi = 0;

    if (total > 0)
        centi = count * 10000 / total;
    my_putchar(letter);
    my_putchar(':');
    my_put_nbr(count);
    my_putstr(" (");
    my_put_nbr(centi / 100);
    my_putchar('.');
    my_putchar('0' + centi / 10 % 10);
    my_putchar('0' + centi % 10);
    my_putstr("%)\n");
}

int rush2(char const *text, char **letters, int count)
{
    int total = count_all_letters(text);
    char const *language = guess_language_name(text, total);
    int i = 0;

    while (i < count) {
        display_letter_line(letters[i][0], count_letter(text, letters[i][0]),
            total);
        i = i + 1;
    }
    my_putstr("=> ");
    my_putstr(language);
    my_putchar('\n');
    return (0);
}
