/*
** EPITECH PROJECT, 2026
** cpool_day07
** File description:
** my_sort_params
*/

void my_putchar(char c);
int my_putstr(char const *str);
int my_strcmp(char const *s1, char const *s2);

static int idx_of_min(int argc, char **argv, int start)
{
    int min = start;
    int j = start + 1;

    while (j < argc) {
        if (my_strcmp(argv[j], argv[min]) < 0) {
            min = j;
        }
        j = j + 1;
    }
    return (min);
}

static void sort_params(int argc, char **argv)
{
    int i = 0;
    int min;
    char *tmp;

    while (i < argc - 1) {
        min = idx_of_min(argc, argv, i);
        if (min != i) {
            tmp = argv[i];
            argv[i] = argv[min];
            argv[min] = tmp;
        }
        i = i + 1;
    }
}

int main(int argc, char **argv)
{
    int i = 0;

    sort_params(argc, argv);
    while (i < argc) {
        my_putstr(argv[i]);
        my_putchar('\n');
        i = i + 1;
    }
    return (0);
}
