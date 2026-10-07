/*
** EPITECH PROJECT, 2026
** cpool_day09
** File description:
** my_show_word_array de référence (libmy de la batterie)
*/

int my_putstr(char const *str);
void my_putchar(char c);

int my_show_word_array(char *const *tab)
{
    int i = 0;

    while (tab[i] != 0) {
        my_putstr(tab[i]);
        my_putchar('\n');
        i++;
    }
    return (0);
}
