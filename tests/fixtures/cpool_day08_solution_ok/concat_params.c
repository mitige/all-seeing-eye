/*
** EPITECH PROJECT, 2026
** cpool_day08
** File description:
** concat_params
*/

#include <stdlib.h>

static int total_length(int argc, char **argv)
{
    int total = 0;
    int i = 0;
    int j = 0;

    while (i < argc) {
        j = 0;
        while (argv[i][j] != '\0') {
            j = j + 1;
        }
        total = total + j;
        i = i + 1;
    }
    if (argc > 0) {
        total = total + argc - 1;
    }
    return (total);
}

static int append_arg(char *out, int k, char const *arg)
{
    int j = 0;

    while (arg[j] != '\0') {
        out[k] = arg[j];
        k = k + 1;
        j = j + 1;
    }
    return (k);
}

char *concat_params(int argc, char **argv)
{
    int total = total_length(argc, argv);
    char *out = NULL;
    int i = 0;
    int k = 0;

    out = malloc(sizeof(char) * (total + 1));
    if (out == NULL) {
        return (NULL);
    }
    while (i < argc) {
        k = append_arg(out, k, argv[i]);
        if (i + 1 < argc) {
            out[k] = '\n';
            k = k + 1;
        }
        i = i + 1;
    }
    out[k] = '\0';
    return (out);
}
