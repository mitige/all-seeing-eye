/*
** EPITECH PROJECT, 2026
** cpool_day04
** File description:
** main de test pour my_sort_int_array
*/

#include <stdio.h>

void my_sort_int_array(int *array, int size);

static void print_array(int *array, int size)
{
    int i = 0;

    while (i < size) {
        printf("%d", array[i]);
        if (i + 1 < size)
            printf(" ");
        i = i + 1;
    }
    printf("\n");
}

int main(void)
{
    int t1[] = {5, 4, 3, 2, 1};
    int t2[] = {1, 2, 3, 4, 5};
    int t3[] = {42};
    int t4[] = {0, -1, 5, -42, 2147483647, -2147483647 - 1};
    int t5[] = {3, 3, 1, 1, 2, 2};

    my_sort_int_array(t1, 5);
    print_array(t1, 5);
    my_sort_int_array(t2, 5);
    print_array(t2, 5);
    my_sort_int_array(t3, 1);
    print_array(t3, 1);
    my_sort_int_array(t4, 6);
    print_array(t4, 6);
    my_sort_int_array(t5, 6);
    print_array(t5, 6);
    my_sort_int_array(t1, 0);
    printf("ok\n");
    return (0);
}
