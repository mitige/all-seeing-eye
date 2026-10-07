/*
** EPITECH PROJECT, 2026
** cpool_day05
** File description:
** main de test pour count_valid_queens_placements
*/

void my_putchar(char c);
int count_valid_queens_placements(int n);

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
    print_result(count_valid_queens_placements(1));
    print_result(count_valid_queens_placements(2));
    print_result(count_valid_queens_placements(3));
    print_result(count_valid_queens_placements(4));
    print_result(count_valid_queens_placements(5));
    print_result(count_valid_queens_placements(6));
    print_result(count_valid_queens_placements(7));
    print_result(count_valid_queens_placements(8));
    print_result(count_valid_queens_placements(10));
    return (0);
}
