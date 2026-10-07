/*
** EPITECH PROJECT, 2026
** cpool_day08
** File description:
** main de test pour my_show_word_array
*/

#include <stddef.h>
#include <unistd.h>

int my_show_word_array(char * const *tab);

int main(void)
{
    char *case1[] = {"The", "Answer", "to", "the", "Great",
        "Question...", "Of", "Life,", "the", "Universe", "and",
        "Everything...", "Is...", "Forty-two,", NULL};
    char *case2[] = {NULL};
    char *case3[] = {"", "x", "", NULL};

    my_show_word_array(case1);
    write(1, "===\n", 4);
    my_show_word_array(case2);
    write(1, "===\n", 4);
    my_show_word_array(case3);
    write(1, "===\n", 4);
    return (0);
}
