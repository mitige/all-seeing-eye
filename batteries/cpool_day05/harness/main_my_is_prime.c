/*
** EPITECH PROJECT, 2026
** cpool_day05
** File description:
** main de test pour my_is_prime
*/

void my_putchar(char c);
int my_is_prime(int nb);

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
    print_result(my_is_prime(-7));
    print_result(my_is_prime(-2));
    print_result(my_is_prime(0));
    print_result(my_is_prime(1));
    print_result(my_is_prime(2));
    print_result(my_is_prime(3));
    print_result(my_is_prime(4));
    print_result(my_is_prime(9));
    print_result(my_is_prime(97));
    print_result(my_is_prime(100));
    print_result(my_is_prime(2147483646));
    print_result(my_is_prime(2147483647));
    return (0);
}
