/*
** EPITECH PROJECT, 2026
** cpool_rush1
** File description:
** rush-1-2 : carre etoiles, coins slash et antislash
*/

#include <unistd.h>

void my_putchar(char c);

static char corner_at(int col, int row, int y)
{
    if (row == 1 && col == 1) {
        return ('/');
    }
    if (row == 1) {
        return ('\\');
    }
    if (col == 1 && row == y) {
        return ('\\');
    }
    return ('/');
}

static char char_at(int col, int row, int x, int y)
{
    if (x == 1 || y == 1) {
        return ('*');
    }
    if ((col == 1 || col == x) && (row == 1 || row == y)) {
        return (corner_at(col, row, y));
    }
    if (col == 1 || col == x || row == 1 || row == y) {
        return ('*');
    }
    return (' ');
}

static void draw(int x, int y)
{
    int col;
    int row;

    row = 1;
    while (row <= y) {
        col = 1;
        while (col <= x) {
            my_putchar(char_at(col, row, x, y));
            col = col + 1;
        }
        my_putchar('\n');
        row = row + 1;
    }
}

void rush(int x, int y)
{
    if (x <= 0 || y <= 0) {
        write(2, "Invalid size\n", 13);
        return;
    }
    draw(x, y);
}
