/*
** EPITECH PROJECT, 2026
** all-seeing-eye
** File description:
** Contrat de layout imposé par la batterie cpool_day13 — pattern
** cpool_day09 info_param : le harness définit la structure À
** L'IDENTIQUE de ce que le sujet fait attendre (deux TUs, deux
** définitions compatibles). Champs width, height, pixels DANS CET
** ORDRE ; pixels = tableau sfUint8 RGBA contigu de w * h * 4 octets.
*/

#ifndef D13_FRAMEBUFFER_H
    #define D13_FRAMEBUFFER_H
    #include <SFML/Graphics.h>

typedef struct framebuffer {
    unsigned int width;
    unsigned int height;
    sfUint8 *pixels;
} framebuffer_t;

#endif
