/*
** EPITECH PROJECT, 2026
** cpool_day09
** File description:
** main de test pour my_macro_abs.h (macro ABS)
*/

/*
** La macro ABS est éprouvée sur : zéro, négatif, positif, borne basse
** sans UB (INT_MIN exclu : -INT_MIN est indéfinissable en int), et
** surtout une EXPRESSION composée (1 - 4) qui piège les macros sans
** parenthèses autour du paramètre : « v < 0 ? -v : v » donnerait -5.
*/

#include <stdio.h>
#include "my_macro_abs.h"

int main(void)
{
    printf("%d\n", ABS(0));
    printf("%d\n", ABS(-5));
    printf("%d\n", ABS(5));
    printf("%d\n", ABS(-2147483647));
    printf("%d\n", ABS(1 - 4));
    printf("%d\n", ABS(2 + 2));
    return (0);
}
