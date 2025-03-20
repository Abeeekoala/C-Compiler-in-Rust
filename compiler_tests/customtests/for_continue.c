int for_test() {
    int i = 0;
    for (i=0; i<5; i++) {
        if (i == 3) break;
        if (i == 1) continue;
    }
    return i;
}
