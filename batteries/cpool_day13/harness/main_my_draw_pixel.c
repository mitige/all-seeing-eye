/*
** EPITECH PROJECT, 2026
** all-seeing-eye
** File description:
** Harness my_draw_pixel : framebuffer 80x60 alloué et zéroé par le
** harness (pas par la delivery — on ne teste QU'elle), trois pixels
** dessinés, octets RGBA vérifiés à l'offset (y * width + x) * 4, puis
** scan plein cadre : exactement 2 pixels allumés (le {0,0,0,0} écrit
** des zéros — il éprouve l'offset sans allumer). Le scan attrape les
** effets de bord (mauvais stride, écriture hors cible).
*/

#include <stdio.h>
#include <stdlib.h>
#include "d13_framebuffer.h"

void my_draw_pixel(framebuffer_t *framebuffer, unsigned int x,
    unsigned int y, sfColor color);

static framebuffer_t *blank_fb(unsigned int w, unsigned int h)
{
    framebuffer_t *fb = malloc(sizeof(*fb));

    fb->width = w;
    fb->height = h;
    fb->pixels = calloc((unsigned long)w * h * 4, 1);
    return (fb);
}

static int expect_px(framebuffer_t const *fb, unsigned int x,
    unsigned int y, sfColor const *c)
{
    unsigned long o = ((unsigned long)y * fb->width + x) * 4;

    if (fb->pixels[o] == c->r && fb->pixels[o + 1] == c->g
        && fb->pixels[o + 2] == c->b && fb->pixels[o + 3] == c->a)
        return (0);
    printf("KO: pixel(%u,%u) incorrect (attendu %u,%u,%u,%u)\n", x, y,
        (unsigned int)c->r, (unsigned int)c->g, (unsigned int)c->b,
        (unsigned int)c->a);
    return (1);
}

static int expect_lit(framebuffer_t const *fb, int attendu)
{
    unsigned long total = (unsigned long)fb->width * fb->height;
    unsigned long i;
    int n = 0;

    for (i = 0; i < total; i++) {
        if (fb->pixels[i * 4] != 0 || fb->pixels[i * 4 + 1] != 0
            || fb->pixels[i * 4 + 2] != 0 || fb->pixels[i * 4 + 3] != 0)
            n++;
    }
    if (n != attendu) {
        printf("KO: %d pixels allumés, attendu %d\n", n, attendu);
        return (1);
    }
    return (0);
}

int main(void)
{
    framebuffer_t *fb = blank_fb(80, 60);
    sfColor red = {255, 0, 0, 255};
    sfColor weird = {1, 2, 3, 4};
    sfColor zero = {0, 0, 0, 0};
    int ko = 0;

    my_draw_pixel(fb, 0, 0, zero);
    my_draw_pixel(fb, 10, 10, red);
    my_draw_pixel(fb, 79, 59, weird);
    ko |= expect_px(fb, 0, 0, &zero);
    ko |= expect_px(fb, 10, 10, &red);
    ko |= expect_px(fb, 79, 59, &weird);
    ko |= expect_lit(fb, 2);
    free(fb->pixels);
    free(fb);
    if (ko != 0)
        return (1);
    printf("OK\n");
    return (0);
}
