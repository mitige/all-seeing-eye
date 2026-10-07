/*
** EPITECH PROJECT, 2026
** cpool_day13
** File description:
** task04 - charge une image BMP 24 bits et l'affiche en fenêtre
*/

#include <SFML/Graphics.h>
#include <stdio.h>
#include <stdlib.h>

static unsigned int read_u32le(unsigned char const *p)
{
    unsigned int value = p[0];

    value |= (unsigned int)p[1] << 8;
    value |= (unsigned int)p[2] << 16;
    value |= (unsigned int)p[3] << 24;
    return (value);
}

static void read_row(FILE *file, sfUint8 *pixels, unsigned int w,
    unsigned int y)
{
    unsigned int pad = (4 - (w * 3) % 4) % 4;
    unsigned int x;
    unsigned int i;

    for (x = 0; x < w; x++) {
        pixels[(y * w + x) * 4 + 2] = fgetc(file);
        pixels[(y * w + x) * 4 + 1] = fgetc(file);
        pixels[(y * w + x) * 4 + 0] = fgetc(file);
        pixels[(y * w + x) * 4 + 3] = 255;
    }
    for (i = 0; i < pad; i++)
        fgetc(file);
}

static sfUint8 *alloc_pixels(FILE *file, unsigned char const *header,
    unsigned int *w, unsigned int *h)
{
    sfUint8 *pixels;

    *w = read_u32le(header + 18);
    *h = read_u32le(header + 22);
    pixels = malloc(sizeof(*pixels) * *w * *h * 4);
    if (pixels == NULL)
        return (NULL);
    if (fseek(file, (long)read_u32le(header + 10), SEEK_SET) != 0) {
        free(pixels);
        return (NULL);
    }
    return (pixels);
}

static sfUint8 *load_bmp(char const *path, unsigned int *w,
    unsigned int *h)
{
    FILE *file = fopen(path, "rb");
    unsigned char header[54];
    sfUint8 *pixels;
    unsigned int y;

    if (file == NULL || fread(header, 1, 54, file) != 54)
        return (NULL);
    pixels = alloc_pixels(file, header, w, h);
    if (pixels == NULL) {
        fclose(file);
        return (NULL);
    }
    for (y = 0; y < *h; y++)
        read_row(file, pixels, *w, *h - 1 - y);
    fclose(file);
    return (pixels);
}

static void handle_events(sfRenderWindow *window)
{
    sfEvent event;

    while (sfRenderWindow_pollEvent(window, &event)) {
        if (event.type == sfEvtClosed)
            sfRenderWindow_close(window);
    }
}

static void display_image(sfRenderWindow *window, sfUint8 const *pixels,
    unsigned int w, unsigned int h)
{
    sfTexture *texture = sfTexture_create(w, h);
    sfSprite *sprite = sfSprite_create();
    sfColor black = {0, 0, 0, 255};

    sfTexture_updateFromPixels(texture, pixels, w, h, 0, 0);
    sfSprite_setTexture(sprite, texture, sfTrue);
    while (sfRenderWindow_isOpen(window)) {
        handle_events(window);
        sfRenderWindow_clear(window, black);
        sfRenderWindow_drawSprite(window, sprite, NULL);
        sfRenderWindow_display(window);
    }
    sfTexture_destroy(texture);
    sfSprite_destroy(sprite);
}

int main(int argc, char **argv)
{
    sfVideoMode mode = {800, 600, 32};
    sfRenderWindow *window;
    sfUint8 *pixels;
    unsigned int w;
    unsigned int h;

    if (argc != 2)
        return (84);
    pixels = load_bmp(argv[1], &w, &h);
    if (pixels == NULL)
        return (84);
    window = sfRenderWindow_create(mode, "image", sfClose, NULL);
    if (window == NULL)
        return (84);
    display_image(window, pixels, w, h);
    sfRenderWindow_destroy(window);
    free(pixels);
    return (0);
}
