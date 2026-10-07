/*
** EPITECH PROJECT, 2026
** all-seeing-eye
** File description:
** Harness my_draw_square : bloc 5x5 bleu à (10,10) + carré 1x1 rouge
** à (0,0) dans un 50x50 alloué/zéroé par le harness ; octets RGBA
** vérifiés, scan plein cadre (26 pixels allumés attendus).
** La delivery est ISOLÉE (my_draw_square.c seul) : si elle appelle
** my_draw_pixel sans le définir, l'implémentation de référence
** ci-dessous — symbole FAIBLE — satisfait le link et le test ; une
** définition étudiante (symbole fort, ex. copiée dans la delivery)
** prime toujours sur la faible.
*/

#include <stdio.h>
#include <stdlib.h>
#include "d13_framebuffer.h"

void my_draw_square(framebuffer_t *framebuffer, sfVector2u position,
    unsigned int size, sfColor color);

__attribute__((weak))
void my_draw_pixel(framebuffer_t *fb, unsigned int x, unsigned int y,
    sfColor color)
{
    unsigned long o;

    if (x >= fb->width || y >= fb->height)
        return;
    o = ((unsigned long)y * fb->width + x) * 4;
    fb->pixels[o] = color.r;
    fb->pixels[o + 1] = color.g;
    fb->pixels[o + 2] = color.b;
    fb->pixels[o + 3] = color.a;
}

static framebuffer_t *blank_fb(unsigned int w, unsigned int h)
{
    framebuffer_t *fb = malloc(sizeof(*fb));

    fb->width = w;
    fb->height = h;
    fb->pixels = calloc((unsigned long)w * h * 4, 1);
    return (fb);
}

static int px_mismatch(framebuffer_t const *fb, unsigned int x,
    unsigned int y, sfColor const *c)
{
    unsigned long o = ((unsigned long)y * fb->width + x) * 4;

    return (fb->pixels[o] != c->r || fb->pixels[o + 1] != c->g
        || fb->pixels[o + 2] != c->b || fb->pixels[o + 3] != c->a);
}

static int expect_square(framebuffer_t const *fb, sfVector2u const *pos,
    unsigned int size, sfColor const *c)
{
    unsigned int x;
    unsigned int y;
    int bad = 0;

    for (y = pos->y; y < pos->y + size; y++) {
        for (x = pos->x; x < pos->x + size; x++) {
            bad += px_mismatch(fb, x, y, c);
        }
    }
    if (bad != 0) {
        printf("KO: %d pixels du carré (%u,%u) taille %u incorrects\n",
            bad, pos->x, pos->y, size);
        return (1);
    }
    return (0);
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
    framebuffer_t *fb = blank_fb(50, 50);
    sfVector2u pos = {10, 10};
    sfVector2u origin = {0, 0};
    sfColor blue = {0, 0, 255, 255};
    sfColor red = {255, 0, 0, 255};
    int ko = 0;

    my_draw_square(fb, pos, 5, blue);
    my_draw_square(fb, origin, 1, red);
    ko |= expect_square(fb, &pos, 5, &blue);
    ko |= expect_square(fb, &origin, 1, &red);
    ko |= expect_lit(fb, 26);
    free(fb->pixels);
    free(fb);
    if (ko != 0)
        return (1);
    printf("OK\n");
    return (0);
}
