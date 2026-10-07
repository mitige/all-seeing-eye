/*
** EPITECH PROJECT, 2026
** cpool_workshoplib
** File description:
** main de test du groupe math (swap, sort, power, sqrt, prime, getnbr)
*/

void my_putchar(char c);
void my_swap(int *a, int *b);
void my_sort_int_array(int *tab, int size);
int my_compute_power_rec(int nb, int power);
int my_compute_square_root(int nb);
int my_is_prime(int nb);
int my_find_prime_sup(int nb);
int my_getnbr(char const *str);

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

static void put_int_sp(int n)
{
    put_int(n);
    my_putchar(' ');
}

static void end_sort_line(void)
{
    my_putchar('E');
    my_putchar('\n');
}

static void test_swap_sort(void)
{
    int a = 1;
    int b = 2;
    int t[7] = {5, -3, 42, 0, 7, -100, 42};
    int one[1] = {9};
    int i = 0;

    my_swap(&a, &b);
    put_int_sp(a);
    put_int(b);
    my_putchar('\n');
    my_sort_int_array(t, 7);
    while (i < 7) {
        put_int_sp(t[i]);
        i = i + 1;
    }
    my_sort_int_array(one, 1);
    put_int(one[0]);
    my_sort_int_array(t, 0);
    end_sort_line();
}

static void test_math(void)
{
    put_int_sp(my_compute_power_rec(2, 10));
    put_int_sp(my_compute_power_rec(3, 0));
    put_int_sp(my_compute_power_rec(5, 1));
    put_int_sp(my_compute_power_rec(2, -1));
    put_int_sp(my_compute_power_rec(-2, 3));
    put_int(my_compute_power_rec(0, 0));
    my_putchar('\n');
    put_int_sp(my_compute_square_root(16));
    put_int_sp(my_compute_square_root(17));
    put_int_sp(my_compute_square_root(0));
    put_int_sp(my_compute_square_root(1));
    put_int_sp(my_compute_square_root(-4));
    put_int(my_compute_square_root(2147395600));
    my_putchar('\n');
}

static void test_prime(void)
{
    int primes[8] = {0, 1, 2, 3, 4, 97, 100, -7};
    int sup[6] = {0, 2, 14, 97, 100, -5};
    int i = 0;

    while (i < 8) {
        put_int_sp(my_is_prime(primes[i]));
        i = i + 1;
    }
    my_putchar('\n');
    i = 0;
    while (i < 6) {
        put_int_sp(my_find_prime_sup(sup[i]));
        i = i + 1;
    }
    my_putchar('\n');
}

static void test_getnbr_parse(void)
{
    put_int(my_getnbr("42"));
    my_putchar('\n');
    put_int(my_getnbr("-42"));
    my_putchar('\n');
    put_int(my_getnbr("+---+--++---+---+---+-42"));
    my_putchar('\n');
    put_int(my_getnbr("42a43"));
    my_putchar('\n');
    put_int(my_getnbr("abc"));
    my_putchar('\n');
    put_int(my_getnbr(""));
    my_putchar('\n');
}

static void test_getnbr_limits(void)
{
    put_int(my_getnbr("11000000000000000000000042"));
    my_putchar('\n');
    put_int(my_getnbr("-1000000000000000000000042"));
    my_putchar('\n');
    put_int(my_getnbr("2147483647"));
    my_putchar('\n');
    put_int(my_getnbr("-2147483648"));
    my_putchar('\n');
    put_int(my_getnbr("2147483648"));
    my_putchar('\n');
}

int main(void)
{
    test_swap_sort();
    test_math();
    test_prime();
    test_getnbr_parse();
    test_getnbr_limits();
    return (0);
}
