int while_test() {
    int i = 3;
    while (i > 0) {
        if (i == 1) break;
        i--;
    }
    while (i < 5) {
        i++;
        if (i == 3) continue;
        i++;
    }

    return i;
}
