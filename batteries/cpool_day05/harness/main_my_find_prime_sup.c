/*
** EPITECH PROJECT, 2026
** cpool_day05
** File description:
** main de test pour my_find_prime_sup
*/

void my_putchar(char c);
int my_find_prime_sup(int nb);

static void print_nbr(int nb)
{
    unsigned int n;

    if (nb < 0) {
        my_putchar('-');
        n = 0 - (unsigned int)nb;
    } else {
        n = (unsigned int)nb;
    }
    if (n > 9) {
        print_nbr((int)(n / 10));
    }
    my_putchar('0' + (int)(n % 10));
}

static void print_result(int value)
{
    print_nbr(value);
    my_putchar('\n');
}

int main(void)
{
    print_result(my_find_prime_sup(-5));
    print_result(my_find_prime_sup(0));
    print_result(my_find_prime_sup(1));
    print_result(my_find_prime_sup(2));
    print_result(my_find_prime_sup(3));
    print_result(my_find_prime_sup(4));
    print_result(my_find_prime_sup(14));
    print_result(my_find_prime_sup(100));
    print_result(my_find_prime_sup(2147483640));
    return (0);
}
