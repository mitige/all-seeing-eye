/*
** EPITECH PROJECT, 2026
** cpool_day13
** File description:
** my_draw_square : carré plein de size x size pixels
*/

#include "framebuffer.h"

void my_draw_square(framebuffer_t *framebuffer, sfVector2u position,
    unsigned int size, sfColor color)
{
    unsigned int dx;
    unsigned int dy;
    unsigned long offset;

    for (dy = 0; dy < size; dy++) {
        for (dx = 0; dx < size; dx++) {
            offset = ((unsigned long)(position.y + dy)
                * framebuffer->width + position.x + dx) * 4;
            framebuffer->pixels[offset] = color.r;
            framebuffer->pixels[offset + 1] = color.g;
            framebuffer->pixels[offset + 2] = color.b;
            framebuffer->pixels[offset + 3] = color.a;
        }
    }
}
