/*
** EPITECH PROJECT, 2026
** all-seeing-eye
** File description:
** Harness framebuffer_create : vérifie retour non NULL, champs
** width/height, pixels non NULL et TOUS les octets à zéro
** (comportement canonique : calloc), sur 80x60 et 1x1. En cas de KO,
** on ne libère rien : un pixels invalide ferait crasher free() et
** masquerait le diagnostic déjà imprimé — la fuite meurt avec le
** processus.
*/

#include <stdio.h>
#include <stdlib.h>
#include "d13_framebuffer.h"

framebuffer_t *framebuffer_create(unsigned int width, unsigned int height);

/*
** Souille le heap AVANT tout appel : deux blocs des tailles testées
** sont remplis de 0xAB puis libérés — le malloc suivant de même
** taille les réutilise AVEC leurs ordures (tcache LIFO). Un create
** qui ne zéroie pas (malloc nu) échoue alors de façon DÉTERMINISTE ;
** un calloc (comportement canonique attendu) nettoie et passe.
*/
static void salir_heap(void)
{
    unsigned char *petit = malloc(4);
    unsigned char *grand = malloc(80 * 60 * 4);
    unsigned long i;

    for (i = 0; i < 4; i++)
        petit[i] = 0xAB;
    for (i = 0; i < 80 * 60 * 4; i++)
        grand[i] = 0xAB;
    free(petit);
    free(grand);
}

static int check_zeroed(framebuffer_t const *fb)
{
    unsigned long total = (unsigned long)fb->width * fb->height * 4;
    unsigned long i;

    for (i = 0; i < total; i++) {
        if (fb->pixels[i] != 0) {
            printf("KO: pixels[%lu] = %u, attendu 0\n", i,
                (unsigned int)fb->pixels[i]);
            return (1);
        }
    }
    return (0);
}

static int check_fb(framebuffer_t *fb, unsigned int w, unsigned int h)
{
    if (fb == NULL) {
        printf("KO: framebuffer_create(%u, %u) a renvoyé NULL\n", w, h);
        return (1);
    }
    if (fb->width != w || fb->height != h) {
        printf("KO: width/height = %u/%u, attendu %u/%u\n", fb->width,
            fb->height, w, h);
        return (1);
    }
    if (fb->pixels == NULL) {
        printf("KO: pixels est NULL (%ux%u)\n", w, h);
        return (1);
    }
    return (check_zeroed(fb));
}

int main(void)
{
    framebuffer_t *a;
    framebuffer_t *b;
    int ko = 0;

    salir_heap();
    a = framebuffer_create(80, 60);
    b = framebuffer_create(1, 1);
    ko |= check_fb(a, 80, 60);
    ko |= check_fb(b, 1, 1);
    if (ko != 0)
        return (1);
    free(a->pixels);
    free(a);
    free(b->pixels);
    free(b);
    printf("OK\n");
    return (0);
}
