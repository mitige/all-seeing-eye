/*
** EPITECH PROJECT, 2026
** cpool_workshoplib
** File description:
** main de test du groupe is (str_isalpha, isnum, islower, isupper, isprintable)
*/

void my_putchar(char c);
int my_str_isalpha(char const *str);
int my_str_isnum(char const *str);
int my_str_islower(char const *str);
int my_str_isupper(char const *str);
int my_str_isprintable(char const *str);

static void put_str(char const *s)
{
    int i = 0;

    while (s[i] != '\0') {
        my_putchar(s[i]);
        i = i + 1;
    }
}

static void put_bool(int x)
{
    my_putchar(x ? '1' : '0');
    my_putchar(' ');
}

static void test_isprintable(void)
{
    char np1[] = {'a', '\t', 'b', '\0'};
    char np2[] = {'a', 127, '\0'};
    char np3[] = {'a', 1, '\0'};

    put_str("isprintable: ");
    put_bool(my_str_isprintable("abc ABC 123!~"));
    put_bool(my_str_isprintable(np1));
    put_bool(my_str_isprintable(np2));
    put_bool(my_str_isprintable(np3));
    put_bool(my_str_isprintable(""));
    my_putchar('\n');
}

int main(void)
{
    put_str("isalpha: ");
    put_bool(my_str_isalpha("abcdefXYZ"));
    put_bool(my_str_isalpha("abc1"));
    put_bool(my_str_isalpha(""));
    my_putchar('\n');
    put_str("isnum: ");
    put_bool(my_str_isnum("0123456789"));
    put_bool(my_str_isnum("42a"));
    put_bool(my_str_isnum(""));
    my_putchar('\n');
    put_str("islower: ");
    put_bool(my_str_islower("abc"));
    put_bool(my_str_islower("abC"));
    put_bool(my_str_islower(""));
    my_putchar('\n');
    put_str("isupper: ");
    put_bool(my_str_isupper("ABC"));
    put_bool(my_str_isupper("ABc"));
    put_bool(my_str_isupper(""));
    my_putchar('\n');
    test_isprintable();
    return (0);
}
