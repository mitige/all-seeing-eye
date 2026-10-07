/*
** EPITECH PROJECT, 2026
** cpool_day07
** File description:
** main de test pour my_strcat
*/

void my_putchar(char c);
char *my_strcat(char *dest, char const *src);

static void put_str(char const *s)
{
    int i = 0;

    while (s[i] != '\0') {
        my_putchar(s[i]);
        i = i + 1;
    }
}

static void check(char *dest, char const *src)
{
    char *ret = my_strcat(dest, src);

    put_str(dest);
    my_putchar('\n');
    if (ret == dest) {
        my_putchar('Y');
    } else {
        my_putchar('N');
    }
    my_putchar('\n');
}

int main(void)
{
    char d1[32] = "Hello ";
    char d2[8] = "";
    char d3[16] = "abc";

    check(d1, "world");
    check(d2, "abc");
    check(d3, "");
    return (0);
}
