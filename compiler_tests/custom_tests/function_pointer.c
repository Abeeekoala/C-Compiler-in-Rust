int add(int a, int b)
{
    return a + b;
}

int subtract(int a, int b)
{
    return a - b;
}

int f()
{
    int (*operation)(int, int);

    operation = add;
    int result1 = operation(5, 3);

    operation = subtract;
    int result2 = operation(10, 4);

    return (result1 == 8 && result2 == 6) ? 1 : 0;
}
