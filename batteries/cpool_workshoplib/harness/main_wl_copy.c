/*
** EPITECH PROJECT, 2026
** cpool_workshoplib
** File description:
** main de test du groupe copy (my_strcpy, my_strncpy, my_strcat, my_strncat)
*/

void my_putchar(char c);
char *my_strcpy(char *dest, char const *src);
char *my_strncpy(char *dest, char const *src, int n);
char *my_strcat(char *dest, char const *src);
char *my_strncat(char *dest, char const *src, int nb);

static void put_str(char const *s)
{
    int i = 0;

    while (s[i] != '\0') {
        my_putchar(s[i]);
        i = i + 1;
    }
}

static void report(char const *dest, char const *ret)
{
    put_str(dest);
    my_putchar(' ');
    my_putchar(ret == dest ? 'Y' : 'N');
    my_putchar('\n');
}

static void test_strcpy(void)
{
    char d1[32];
    char *r;

    r = my_strcpy(d1, "Hello, World!");
    report(d1, r);
    r = my_strcpy(d1, "");
    my_putchar('[');
    put_str(d1);
    my_putchar(']');
    my_putchar(' ');
    my_putchar(r == d1 ? 'Y' : 'N');
    my_putchar('\n');
}

static void test_strncpy(void)
{
    char d2[8] = {0};
    char d3[8] = "XXXXXXX";
    char *r;

    r = my_strncpy(d2, "HelloWorld", 5);
    report(d2, r);
    r = my_strncpy(d3, "ab", 5);
    report(d3, r);
}

static void test_strcat(void)
{
    char d4[32] = "Hello";
    char d5[32] = "";
    char *r;

    r = my_strcat(d4, " World");
    report(d4, r);
    r = my_strcat(d4, "");
    report(d4, r);
    r = my_strcat(d5, "abc");
    report(d5, r);
}

static void test_strncat(void)
{
    char d6[32] = "ab";
    char d7[32] = "ab";
    char d8[32] = "ab";
    char *r;

    r = my_strncat(d6, "cdefg", 3);
    report(d6, r);
    r = my_strncat(d7, "xy", 10);
    report(d7, r);
    r = my_strncat(d8, "xy", 0);
    report(d8, r);
}

int main(void)
{
    test_strcpy();
    my_putchar('\n');
    test_strncpy();
    my_putchar('\n');
    test_strcat();
    my_putchar('\n');
    test_strncat();
    return (0);
}
