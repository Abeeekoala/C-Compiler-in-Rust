struct Point {
    int x;
    int y;
    double z;
};

int f()
{
    struct Point p;
    return sizeof(p);
}