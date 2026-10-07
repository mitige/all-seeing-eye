/*
** EPITECH PROJECT, 2026
** cpool_day13
** File description:
** task01 - ouvre une fenêtre 800x600 et la garde ouverte
*/

#include <SFML/Graphics.h>
#include <stddef.h>

static void handle_events(sfRenderWindow *window)
{
    sfEvent event;

    while (sfRenderWindow_pollEvent(window, &event)) {
        if (event.type == sfEvtClosed)
            sfRenderWindow_close(window);
        if (event.type == sfEvtKeyPressed && event.key.code == sfKeyEscape)
            sfRenderWindow_close(window);
    }
}

int main(void)
{
    sfVideoMode mode = {800, 600, 32};
    sfColor black = {0, 0, 0, 255};
    sfRenderWindow *window;

    window = sfRenderWindow_create(mode, "day13", sfClose, NULL);
    if (window == NULL)
        return (84);
    while (sfRenderWindow_isOpen(window)) {
        handle_events(window);
        sfRenderWindow_clear(window, black);
        sfRenderWindow_display(window);
    }
    sfRenderWindow_destroy(window);
    return (0);
}
