struct Node {
    int value;
    struct Node* next;
};

int sum_list() {
    struct Node n3 = {30, 0};
    struct Node n2 = {20, &n3};
    struct Node n1 = {10, &n2};

    struct Node* current = &n1;
    int sum = 0;

    while (current != 0) {
        sum += current->value;
        current = current->next;
    }

    return sum; // Should be 60
}

int main() {
    return sum_list();
}