/*
** EPITECH PROJECT, 2026
** cpool_workshoplib
** File description:
** main de test du groupe case (strupcase, strlowcase, strcapitalize)
*/

void my_putchar(char c);
char *my_strupcase(char *str);
char *my_strlowcase(char *str);
char *my_strcapitalize(char *str);

static void put_str(char const *s)
{
    int i = 0;

    while (s[i] != '\0') {
        my_putchar(s[i]);
        i = i + 1;
    }
}

static void report(char *str, char *ret)
{
    put_str(str);
    my_putchar(' ');
    my_putchar(ret == str ? 'Y' : 'N');
    my_putchar('\n');
}

static void test_upcase(void)
{
    char u1[] = "hello World42!";
    char u2[] = "";

    report(u1, my_strupcase(u1));
    my_putchar('[');
    put_str(my_strupcase(u2));
    my_putchar(']');
    my_putchar('\n');
}

static void test_lowcase(void)
{
    char l1[] = "HeLLo WoRLD42!";

    report(l1, my_strlowcase(l1));
}

static void test_capitalize(void)
{
    char c1[] = "hey, how are you? 42WORds forty-two; fifty+one";
    char c2[] = "HELLO";

    report(c1, my_strcapitalize(c1));
    report(c2, my_strcapitalize(c2));
}

int main(void)
{
    test_upcase();
    test_lowcase();
    test_capitalize();
    return (0);
}
