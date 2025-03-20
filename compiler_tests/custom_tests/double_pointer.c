int f()
{
    int value = 42;
    int *ptr = &value;
    int **ptr_to_ptr = &ptr;

    **ptr_to_ptr = 100; // Change value through double pointer

    return value;
}
