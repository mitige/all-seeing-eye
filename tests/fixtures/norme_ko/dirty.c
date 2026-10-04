int my_sum(int a, int b, int c, int d, int e)
{
    // total
    a += 1; b += 1;  
    goto end;
end:
    return a + b + c + d + e;
}
