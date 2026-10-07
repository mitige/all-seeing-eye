/*
** EPITECH PROJECT, 2026
** cpool_rush2
** File description:
** rush2 core prototypes
*/

#ifndef RUSH2_H_
    #define RUSH2_H_

int rush2(char const *text, char **letters, int count);
int is_letter(char c);
char to_lower(char c);
int count_letter(char const *text, char letter);
int count_all_letters(char const *text);
char const *guess_language_name(char const *text, int total);

#endif
