/*
** EPITECH PROJECT, 2026
** cpool_day07
** File description:
** my_swap
*/

void my_swap(int *a, int *b)
{
    int tmp;

    tmp = *a;
    *a = *b;
    *b = tmp;
}
