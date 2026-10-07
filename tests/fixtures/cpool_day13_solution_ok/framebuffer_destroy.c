/*
** EPITECH PROJECT, 2026
** cpool_day13
** File description:
** framebuffer_destroy : libère le tableau de pixels puis le framebuffer
*/

#include <stdlib.h>
#include "framebuffer.h"

void framebuffer_destroy(framebuffer_t *framebuffer)
{
    free(framebuffer->pixels);
    free(framebuffer);
}
