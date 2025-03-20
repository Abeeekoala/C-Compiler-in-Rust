int f()
{
    int i = 42;
    float f = 3.14;

    void *vptr;

    vptr = &i;
    int *iptr = (int *)vptr;

    vptr = &f;
    float *fptr = (float *)vptr;

    return (*iptr == 42 && *fptr > 3.1 && *fptr < 3.2) ? 1 : 0;
}
