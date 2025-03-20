double f(double x, int n)
{
    double acc=1.0;
    int i=0;
    while(i<n){
        i++;
        acc=acc*x;
    }
    return acc;
}
