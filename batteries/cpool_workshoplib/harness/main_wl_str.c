/*
** EPITECH PROJECT, 2026
** cpool_workshoplib
** File description:
** main de test du groupe str (strlen, revstr, strstr, strcmp, strncmp)
*/

void my_putchar(char c);
int my_strlen(char const *str);
char *my_revstr(char *str);
char *my_strstr(char *str, char const *to_find);
int my_strcmp(char const *s1, char const *s2);
int my_strncmp(char const *s1, char const *s2, int n);

static void put_str(char const *s)
{
    int i = 0;

    while (s[i] != '\0') {
        my_putchar(s[i]);
        i = i + 1;
    }
}

static void put_int(int n)
{
    unsigned int u;

    if (n < 0) {
        my_putchar('-');
        u = 0 - (unsigned int)n;
    } else {
        u = (unsigned int)n;
    }
    if (u > 9) {
        put_int((int)(u / 10));
    }
    my_putchar('0' + (int)(u % 10));
}

static void put_sign(int x)
{
    if (x > 0) {
        put_int(1);
    } else if (x < 0) {
        put_int(-1);
    } else {
        put_int(0);
    }
    my_putchar(' ');
}

static void test_strlen(void)
{
    put_int(my_strlen(""));
    my_putchar(' ');
    put_int(my_strlen("hello"));
    my_putchar(' ');
    put_int(my_strlen("aaaaaaaaaaaaaaaaaaaa"));
    my_putchar('\n');
}

static void test_revstr(void)
{
    char r1[] = "hello";
    char r2[] = "";
    char r3[] = "abcd";

    put_str(my_revstr(r1));
    my_putchar('\n');
    my_putchar('[');
    put_str(my_revstr(r2));
    my_putchar(']');
    my_putchar('\n');
    put_str(my_revstr(r3));
    my_putchar('\n');
}

static void test_strstr(void)
{
    char s1[] = "hello world";
    char s2[] = "aaa";
    char *r;

    r = my_strstr(s1, "world");
    put_str(r != (char *)0 ? r : "NULL");
    my_putchar('\n');
    r = my_strstr(s1, "monde");
    put_str(r != (char *)0 ? r : "NULL");
    my_putchar('\n');
    r = my_strstr(s1, "");
    put_str(r == s1 ? "Y" : "N");
    my_putchar('\n');
    r = my_strstr(s2, "aa");
    put_str(r == s2 ? "Y" : "N");
    my_putchar('\n');
}

static void test_cmp(void)
{
    put_sign(my_strcmp("abc", "abc"));
    put_sign(my_strcmp("abc", "abd"));
    put_sign(my_strcmp("abd", "abc"));
    put_sign(my_strcmp("", ""));
    put_sign(my_strcmp("abc", "abcd"));
    put_sign(my_strcmp("", "a"));
    my_putchar('\n');
    put_sign(my_strncmp("abc", "abd", 2));
    put_sign(my_strncmp("abc", "abd", 3));
    put_sign(my_strncmp("abc", "abd", 0));
    put_sign(my_strncmp("abcd", "abce", 10));
    put_sign(my_strncmp("ab", "abc", 5));
    put_sign(my_strncmp("b", "a", 1));
    my_putchar('\n');
}

int main(void)
{
    test_strlen();
    test_revstr();
    test_strstr();
    test_cmp();
    return (0);
}
