/*
** EPITECH PROJECT, 2026
** cpool_day07
** File description:
** my_rev_params
*/

void my_putchar(char c);
int my_putstr(char const *str);

int main(int argc, char **argv)
{
    int i = argc - 1;

    while (i >= 0) {
        my_putstr(argv[i]);
        my_putchar('\n');
        i = i - 1;
    }
    return (0);
}
