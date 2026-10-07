/*
** EPITECH PROJECT, 2026
** cpool_day06
** File description:
** main de test pour my_showmem
*/

#include <stdio.h>
#include <string.h>

int my_showmem(char const *str, int size);

static void fill_subject_buffer(char *buf)
{
    memcpy(buf,
        "hey guys show mem is cool you can do some pretty neat stuff"
        "\x00\x0f\x1b\x7f\x05\x2e\x00\x01\x02\x03\x04\x05\x06\x07\x08"
        "\x09\x0e\x0f\x1b\x7f",
        79);
}

int main(void)
{
    char buf[79];
    char small[4] = {'H', 'i', '\t', 'Z'};

    setvbuf(stdout, NULL, _IONBF, 0);
    fill_subject_buffer(buf);
    printf("|%d\n", my_showmem(buf, 79));
    printf("|%d\n", my_showmem(small, 4));
    printf("|%d\n", my_showmem(small, 0));
    return (0);
}
