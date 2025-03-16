int f(int idx1, int idx2, int idx3, int num)
{
    int x[10][100][5];
    x[idx1][idx2][idx3]=num;
    return x[idx1][idx2][idx3];
}