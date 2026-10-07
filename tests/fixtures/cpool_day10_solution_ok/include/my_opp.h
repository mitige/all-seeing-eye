/*
** EPITECH PROJECT, 2026
** cpool_day10
** File description:
** my_opp.h
*/

#ifndef MY_OPP_H
    #define MY_OPP_H

typedef struct s_opp {
    char const *op;
    int (*f)(int, int);
} t_opp;

int my_add(int a, int b);
int my_sub(int a, int b);
int my_mul(int a, int b);
int my_div(int a, int b);
int my_mod(int a, int b);
int my_usage(int a, int b);

#endif
