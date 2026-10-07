/*
** EPITECH PROJECT, 2026
** cpool_day07
** File description:
** main de test pour my_strncat
*/

void my_putchar(char c);
char *my_strncat(char *dest, char const *src, int nb);

static void put_str(char const *s)
{
    int i = 0;

    while (s[i] != '\0') {
        my_putchar(s[i]);
        i = i + 1;
    }
}

static void check(char *dest, char const *src, int nb)
{
    char *ret = my_strncat(dest, src, nb);

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
    char d2[32] = "abc";
    char d3[16] = "";
    char d4[16] = "x";

    check(d1, "world", 3);
    check(d2, "defghij", 10);
    check(d3, "abc", 0);
    check(d4, "yz", 2);
    return (0);
}
