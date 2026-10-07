/*
** EPITECH PROJECT, 2026
** cpool_day13
** File description:
** my_draw_pixel : écrit un pixel RGBA dans le framebuffer
*/

#include "framebuffer.h"

void my_draw_pixel(framebuffer_t *framebuffer, unsigned int x,
    unsigned int y, sfColor color)
{
    unsigned long offset;

    if (x >= framebuffer->width || y >= framebuffer->height)
        return;
    offset = ((unsigned long)y * framebuffer->width + x) * 4;
    framebuffer->pixels[offset] = color.r;
    framebuffer->pixels[offset + 1] = color.g;
    framebuffer->pixels[offset + 2] = color.b;
    framebuffer->pixels[offset + 3] = color.a;
}
