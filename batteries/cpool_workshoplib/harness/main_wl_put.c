/*
** EPITECH PROJECT, 2026
** cpool_workshoplib
** File description:
** main de test du groupe put (my_putchar, my_isneg, my_put_nbr, my_putstr)
*/

void my_putchar(char c);
int my_isneg(int nb);
int my_put_nbr(int nb);
int my_putstr(char const *str);

static void test_isneg(void)
{
    my_isneg(-2147483648);
    my_isneg(-1);
    my_isneg(0);
    my_isneg(42);
    my_putchar('\n');
}

static void test_put_nbr(void)
{
    my_put_nbr(0);
    my_putchar('\n');
    my_put_nbr(42);
    my_putchar('\n');
    my_put_nbr(-42);
    my_putchar('\n');
    my_put_nbr(2147483647);
    my_putchar('\n');
    my_put_nbr(-2147483648);
    my_putchar('\n');
}

int main(void)
{
    my_putchar('A');
    my_putchar('z');
    my_putchar('0');
    my_putchar('\n');
    test_isneg();
    my_putstr("hello libmy\n");
    my_putstr("");
    my_putstr(".\n");
    test_put_nbr();
    return (0);
}
