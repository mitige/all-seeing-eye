/*
** EPITECH PROJECT, 2026
** cpool_day05
** File description:
** main de test pour my_compute_factorial_rec
*/

void my_putchar(char c);
int my_compute_factorial_rec(int nb);

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
    print_result(my_compute_factorial_rec(0));
    print_result(my_compute_factorial_rec(1));
    print_result(my_compute_factorial_rec(5));
    print_result(my_compute_factorial_rec(10));
    print_result(my_compute_factorial_rec(-1));
    print_result(my_compute_factorial_rec(-7));
    print_result(my_compute_factorial_rec(12));
    print_result(my_compute_factorial_rec(13));
    print_result(my_compute_factorial_rec(100));
    return (0);
}
