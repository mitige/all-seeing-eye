/*
** EPITECH PROJECT, 2026
** cpool_day09
** File description:
** swap_endian_color
*/

union color_conv
{
    int color;
    unsigned char bytes[4];
};

int swap_endian_color(int color)
{
    union color_conv src;
    union color_conv swapped;

    src.color = color;
    swapped.bytes[0] = src.bytes[3];
    swapped.bytes[1] = src.bytes[2];
    swapped.bytes[2] = src.bytes[1];
    swapped.bytes[3] = src.bytes[0];
    return (swapped.color);
}
