unsigned f() {

    unsigned x = 2;
    unsigned y = 3;
    unsigned a = 7;

    unsigned d = a / x;

    unsigned r = a % x;

    unsigned m = y * 2;

    unsigned cmp = (x < y) ? 1 : 0;
    return d - r + m - cmp;
}