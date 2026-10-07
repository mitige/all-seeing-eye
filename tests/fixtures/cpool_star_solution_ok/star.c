/*
** EPITECH PROJECT, 2026
** cpool_star
** File description:
** star
*/

void my_putchar(char c);

static void repeat_char(char c, unsigned int count)
{
    unsigned int i;

    i = 0;
    while (i < count) {
        my_putchar(c);
        i = i + 1;
    }
}

static void spike_line(unsigned int size, unsigned int k)
{
    repeat_char(' ', 3 * size - k);
    my_putchar('*');
    if (k > 0) {
        repeat_char(' ', 3 * k - 2);
        my_putchar('*');
    }
    my_putchar('\n');
}

static void bar_line(unsigned int size)
{
    repeat_char('*', 2 * size + 1);
    repeat_char(' ', size * size + 3 - 3 * size);
    repeat_char('*', 2 * size + 1);
    my_putchar('\n');
}

static void middle_line(unsigned int size, unsigned int j)
{
    unsigned int depth;
    unsigned int gap;

    depth = j;
    if (2 * size - 2 - j < depth)
        depth = 2 * size - 2 - j;
    gap = size * size + size + 2 - (size / 2 + 1) * depth;
    repeat_char(' ', 1 + depth);
    my_putchar('*');
    repeat_char(' ', gap);
    my_putchar('*');
    my_putchar('\n');
}

void star(unsigned int size)
{
    unsigned int i = 0;

    if (size == 0)
        return;
    while (i < size) {
        spike_line(size, i);
        i = i + 1;
    }
    bar_line(size);
    i = 0;
    while (i < 2 * size - 1) {
        middle_line(size, i);
        i = i + 1;
    }
    bar_line(size);
    i = size;
    while (i > 0) {
        i = i - 1;
        spike_line(size, i);
    }
}
