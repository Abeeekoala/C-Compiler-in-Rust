int f()
{
    int arr[3] = {5, 10, 15};
    int *ptr = arr;

    int val1 = *ptr;
    ptr++;
    int val2 = *ptr;
    ptr++;
    int val3 = *ptr;
    ptr--;
    int val4 = *ptr;

    return (val1 == 5 && val2 == 10 && val3 == 15 && val4 == 10) ? 1 : 0;
}
