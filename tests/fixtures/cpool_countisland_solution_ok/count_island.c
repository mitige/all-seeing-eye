/*
** EPITECH PROJECT, 2026
** cpool_countisland
** File description:
** count_island - remplace les X par le numero de leur ile
*/

#include <stddef.h>
#include "my.h"

static int world_height(char **world)
{
    int height = 0;

    while (world[height] != NULL) {
        height = height + 1;
    }
    return (height);
}

static int world_width(char **world)
{
    if (world[0] == NULL) {
        return (0);
    }
    return (my_strlen(world[0]));
}

static void fill_island(char **world, int row, int col, char mark)
{
    int height = world_height(world);
    int width = world_width(world);

    if (row < 0 || col < 0 || row >= height || col >= width) {
        return;
    }
    if (world[row][col] != 'X') {
        return;
    }
    world[row][col] = mark;
    fill_island(world, row - 1, col, mark);
    fill_island(world, row + 1, col, mark);
    fill_island(world, row, col - 1, mark);
    fill_island(world, row, col + 1, mark);
}

static int scan_line(char **world, int row, int islands)
{
    int col = 0;
    int found = 0;

    while (world[row][col] != '\0') {
        if (world[row][col] == 'X') {
            fill_island(world, row, col, '0' + islands + found);
            found = found + 1;
        }
        col = col + 1;
    }
    return (found);
}

int count_island(char **world)
{
    int row = 0;
    int islands = 0;

    while (world[row] != NULL) {
        islands = islands + scan_line(world, row, islands);
        row = row + 1;
    }
    return (islands);
}
