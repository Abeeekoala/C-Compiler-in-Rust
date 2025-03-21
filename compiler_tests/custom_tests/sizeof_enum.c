int f()
{
    enum X{
        vv = 13,
        yy = 10,
        zz = 11,
        ww = 12
    };
    return sizeof(enum X) + sizeof(vv);
}