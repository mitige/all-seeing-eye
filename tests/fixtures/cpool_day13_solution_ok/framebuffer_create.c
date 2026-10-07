/*
** EPITECH PROJECT, 2026
** cpool_day13
** File description:
** framebuffer_create : alloue un framebuffer, pixels à zéro
*/

#include <stdlib.h>
#include "framebuffer.h"

framebuffer_t *framebuffer_create(unsigned int width,
    unsigned int height)
{
    framebuffer_t *fb = malloc(sizeof(*fb));

    if (fb == NULL)
        return (NULL);
    fb->width = width;
    fb->height = height;
    fb->pixels = calloc((unsigned long)width * height * 4,
        sizeof(*fb->pixels));
    if (fb->pixels == NULL) {
        free(fb);
        return (NULL);
    }
    return (fb);
}
