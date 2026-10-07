/*
** EPITECH PROJECT, 2026
** cpool_day13
** File description:
** framebuffer : pixels RGBA contigus + dimensions
*/

#ifndef FRAMEBUFFER_H_
    #define FRAMEBUFFER_H_
    #include <SFML/Graphics.h>

typedef struct framebuffer {
    unsigned int width;
    unsigned int height;
    sfUint8 *pixels;
} framebuffer_t;

framebuffer_t *framebuffer_create(unsigned int width,
    unsigned int height);
void framebuffer_destroy(framebuffer_t *framebuffer);

#endif
