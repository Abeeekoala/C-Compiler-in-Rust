// struct_pointers.c
struct Rectangle {
    int width;
    int height;
};

int compute_area(struct Rectangle* rect) {
    return rect->width * rect->height;
}

int compute_perimeter(struct Rectangle* rect) {
    return 2 * (rect->width + rect->height);
}

int x() {
    struct Rectangle r;
    r.width = 5;
    r.height = 10;

    int area = compute_area(&r);
    int perimeter = compute_perimeter(&r);

    return area + perimeter;
}