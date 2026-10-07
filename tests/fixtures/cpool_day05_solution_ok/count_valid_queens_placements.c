/*
** EPITECH PROJECT, 2026
** cpool_day05
** File description:
** count_valid_queens_placements
*/

static int is_safe(int *cols, int row, int col)
{
    int i = 0;

    while (i < row) {
        if (cols[i] == col)
            return (0);
        if (cols[i] - col == row - i || col - cols[i] == row - i)
            return (0);
        i = i + 1;
    }
    return (1);
}

static int solve(int *cols, int row, int n)
{
    int col = 0;
    int total = 0;

    if (row == n)
        return (1);
    while (col < n) {
        if (is_safe(cols, row, col)) {
            cols[row] = col;
            total = total + solve(cols, row + 1, n);
        }
        col = col + 1;
    }
    return (total);
}

int count_valid_queens_placements(int n)
{
    int cols[16];

    if (n <= 0 || n > 16)
        return (0);
    return (solve(cols, 0, n));
}
