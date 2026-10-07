/*
** EPITECH PROJECT, 2026
** cpool_day12
** File description:
** cat
*/

#include <errno.h>
#include <fcntl.h>
#include <unistd.h>

static int my_strlen(char const *str)
{
    int len = 0;

    while (str[len] != '\0')
        len = len + 1;
    return (len);
}

static void put_all(int fd, char const *buf, int len)
{
    int written = 0;
    int ret = 0;

    while (written < len) {
        ret = write(fd, buf + written, len - written);
        if (ret <= 0)
            return;
        written = written + ret;
    }
}

static char const *error_message(int err)
{
    if (err == ENOENT)
        return ("No such file or directory");
    if (err == EACCES)
        return ("Permission denied");
    if (err == EISDIR)
        return ("Is a directory");
    return ("Unknown error");
}

static void print_error(char const *path)
{
    int err = errno;
    char const *msg = error_message(err);

    put_all(2, "cat: ", 5);
    put_all(2, path, my_strlen(path));
    put_all(2, ": ", 2);
    put_all(2, msg, my_strlen(msg));
    put_all(2, "\n", 1);
}

static int cat_fd(int fd, char const *path)
{
    char buffer[30720];
    int n = read(fd, buffer, sizeof(buffer));

    while (n > 0) {
        put_all(1, buffer, n);
        n = read(fd, buffer, sizeof(buffer));
    }
    if (n < 0) {
        print_error(path);
        return (1);
    }
    return (0);
}

static int cat_file(char const *path)
{
    int fd = open(path, O_RDONLY);
    int status = 0;

    if (fd < 0) {
        print_error(path);
        return (1);
    }
    status = cat_fd(fd, path);
    close(fd);
    return (status);
}

int main(int argc, char **argv)
{
    int status = 0;
    int i = 1;

    if (argc == 1 && cat_fd(0, "-") != 0)
        status = 1;
    while (i < argc) {
        if (cat_file(argv[i]) != 0)
            status = 1;
        i = i + 1;
    }
    if (status != 0)
        return (84);
    return (0);
}
