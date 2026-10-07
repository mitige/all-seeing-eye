/*
** EPITECH PROJECT, 2026
** cpool_day04
** File description:
** my_sort_int_array
*/

static void bubble_pass(int *array, int size)
{
    int j;
    int tmp;

    j = 0;
    while (j < size - 1) {
        if (array[j] > array[j + 1]) {
            tmp = array[j];
            array[j] = array[j + 1];
            array[j + 1] = tmp;
        }
        j = j + 1;
    }
}

void my_sort_int_array(int *array, int size)
{
    int i;

    i = 0;
    while (i < size) {
        bubble_pass(array, size);
        i = i + 1;
    }
}
