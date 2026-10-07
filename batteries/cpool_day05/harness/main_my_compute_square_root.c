/*
** EPITECH PROJECT, 2026
** cpool_day05
** File description:
** main de test pour my_compute_square_root
*/

void my_putchar(char c);
int my_compute_square_root(int nb);

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
    print_result(my_compute_square_root(0));
    print_result(my_compute_square_root(1));
    print_result(my_compute_square_root(4));
    print_result(my_compute_square_root(9));
    print_result(my_compute_square_root(16));
    print_result(my_compute_square_root(17));
    print_result(my_compute_square_root(25));
    print_result(my_compute_square_root(-4));
    print_result(my_compute_square_root(2147395600));
    print_result(my_compute_square_root(2147483647));
    return (0);
}
