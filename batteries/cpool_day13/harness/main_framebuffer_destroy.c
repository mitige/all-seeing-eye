/*
** EPITECH PROJECT, 2026
** all-seeing-eye
** File description:
** Harness framebuffer_destroy : interception de free() au link via
** -Wl,--wrap=free (link_flags de la task). Toute référence à free
** dans la delivery est résolue en __wrap_free, qui enregistre le
** pointeur puis relaie à __real_free — la libération a VRAIMENT lieu.
** Attendu : exactement DEUX free (pixels + framebuffer), ordre
** indifférent. Les free internes de libc ne sont pas interceptés
** (résolution interne à libc.so, jamais via les objets du link).
** Un destroy no-op ou partiel est KO ; un double-free crashe (KO
** crashed). Les pointeurs attendus sont mémorisés AVANT l'appel :
** lire fb->pixels après destroy serait un use-after-free.
*/

#include <stdio.h>
#include <stdlib.h>
#include "d13_framebuffer.h"

void framebuffer_destroy(framebuffer_t *framebuffer);
extern void __real_free(void *ptr);

static void *g_freed[8];
static int g_nb_freed = 0;

void __wrap_free(void *ptr)
{
    if (ptr != NULL && g_nb_freed < 8) {
        g_freed[g_nb_freed] = ptr;
        g_nb_freed++;
    }
    __real_free(ptr);
}

static int check_frees(void const *attendu_pixels, void const *attendu_fb)
{
    int vu_pixels = 0;
    int vu_fb = 0;
    int i;

    for (i = 0; i < g_nb_freed; i++) {
        if (g_freed[i] == attendu_pixels)
            vu_pixels = 1;
        if (g_freed[i] == attendu_fb)
            vu_fb = 1;
    }
    if (g_nb_freed == 2 && vu_pixels && vu_fb)
        return (0);
    printf("KO: %d free() interceptés (attendu 2 : pixels=%d fb=%d)\n",
        g_nb_freed, vu_pixels, vu_fb);
    return (1);
}

int main(void)
{
    framebuffer_t *fb = malloc(sizeof(*fb));
    void *attendu_pixels;
    int ko;

    fb->width = 10;
    fb->height = 10;
    fb->pixels = malloc(10 * 10 * 4);
    attendu_pixels = fb->pixels;
    framebuffer_destroy(fb);
    ko = check_frees(attendu_pixels, (void *)fb);
    if (ko != 0)
        return (1);
    printf("OK\n");
    return (0);
}
