int f()
{
    int int_arr[5] = {10, 20, 30, 40, 50};
    int *iptr = int_arr;
    iptr += 2;

    return (*iptr == 30) ? 1 : 0;
}
