/*
** EPITECH PROJECT, 2026
** cpool_workshoplib
** File description:
** main de test du groupe show (my_showstr, my_showmem)
*/

void my_putchar(char c);
int my_showstr(char const *str);
int my_showmem(char const *str, int size);

static void put_str(char const *s)
{
    int i = 0;

    while (s[i] != '\0') {
        my_putchar(s[i]);
        i = i + 1;
    }
}

static void put_ret(int r)
{
    put_str(" R");
    my_putchar('0' + r);
    my_putchar('\n');
}

static void test_showstr(void)
{
    char np[] = {'x', 1, 127, 'z', '\0'};

    put_ret(my_showstr("I like \n ponies!\n"));
    put_ret(my_showstr("a\tb"));
    put_ret(my_showstr(np));
    put_ret(my_showstr(""));
}

static void fill_tail(char *buf)
{
    char const tail[20] = {0x00, 0x0f, 0x1b, 0x7f, 0x05, 0x2e, 0x00,
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0e,
        0x0f, 0x1b, 0x7f};
    int i = 0;

    while (i < 20) {
        buf[59 + i] = tail[i];
        i = i + 1;
    }
}

static void test_showmem(void)
{
    char buf[79] = "hey guys show mem is cool you can do some pretty"
        " neat stuff";

    fill_tail(buf);
    put_ret(my_showmem(buf, 79));
    put_ret(my_showmem("abc", 3));
    put_ret(my_showmem(buf, 0));
}

int main(void)
{
    test_showstr();
    test_showmem();
    return (0);
}
