/*
** EPITECH PROJECT, 2026
** cpool_workshoplib
** File description:
** my_sort_int_array
*/

static int idx_of_min(int *tab, int start, int size)
{
    int min = start;
    int j = start + 1;

    while (j < size) {
        if (tab[j] < tab[min]) {
            min = j;
        }
        j = j + 1;
    }
    return (min);
}

void my_sort_int_array(int *tab, int size)
{
    int i = 0;
    int min;
    int tmp;

    while (i < size - 1) {
        min = idx_of_min(tab, i, size);
        if (min != i) {
            tmp = tab[i];
            tab[i] = tab[min];
            tab[min] = tmp;
        }
        i = i + 1;
    }
}
