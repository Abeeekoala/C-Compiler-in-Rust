void swap(int *a, int *b)
{
    int temp = *a;
    *a = *b;
    *b = temp;
}

int f()
{
    int x = 5;
    int y = 10;

    swap(&x, &y);

    return (x == 10 && y == 5) ? 1 : 0;
}
