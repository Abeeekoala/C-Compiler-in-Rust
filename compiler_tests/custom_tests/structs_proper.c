struct Point {
    int x;
    int y;
};

int compute_distance_squared() {
    struct Point p1;
    struct Point p2;

    p1.x = 3;
    p1.y = 4;

    p2.x = 7;
    p2.y = 9;

    int dx = p2.x - p1.x;
    int dy = p2.y - p1.y;

    return dx*dx + dy*dy;
}