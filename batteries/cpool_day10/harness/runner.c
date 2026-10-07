/*
** EPITECH PROJECT, 2026
** cpool_day10 battery
** File description:
** runner.c — execve passthrough vers le binaire construit par le
** rendu (do_op/do-op, my_advanced_do_op/my_advanced_do-op) : argv[1]
** est le chemin du binaire, argv[1..] devient son argv. L'execve
** remplace le processus : stdout, stderr ET exit code du binaire
** testé sont ceux du processus exécuté par la moulinette.
*/

#include <unistd.h>

extern char **environ;

int main(int argc, char **argv)
{
    if (argc < 2)
        return (84);
    execve(argv[1], argv + 1, environ);
    return (84);
}
