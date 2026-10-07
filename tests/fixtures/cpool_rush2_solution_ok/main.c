/*
** EPITECH PROJECT, 2026
** cpool_rush2
** File description:
** entry point of rush2
*/

#include <unistd.h>
#include "rush2.h"

static int usage_error(void)
{
    static const char message[] = "usage: rush2 text letter [letter ...]\n";

    write(2, message, sizeof(message) - 1);
    return (84);
}

int main(int argc, char **argv)
{
    if (argc < 3)
        return (usage_error());
    return (rush2(argv[1], argv + 2, argc - 2));
}
