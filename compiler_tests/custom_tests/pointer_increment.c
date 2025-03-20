int f()
{
    int arr[3] = {5, 10, 15};
    int *ptr = arr;

    int val1 = *ptr;     // Should be 5
    ptr++;
    int val2 = *ptr;     // Should be 10
    ptr++;
    int val3 = *ptr;     // Should be 15
    ptr--;
    int val4 = *ptr;     // Should be 10 again

    return (val1 == 5 && val2 == 10 && val3 == 15 && val4 == 10) ? 1 : 0;
}
