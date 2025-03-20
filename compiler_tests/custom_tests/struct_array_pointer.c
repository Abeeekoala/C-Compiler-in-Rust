// struct_array.c
struct Product {
    int id;
    int price;
    int quantity;
};

int x() {
    struct Product inventory[3];

    inventory[0].id = 101;
    inventory[0].price = 10;
    inventory[0].quantity = 5;

    inventory[1].id = 202;
    inventory[1].price = 25;
    inventory[1].quantity = 2;

    inventory[2].id = 303;
    inventory[2].price = 15;
    inventory[2].quantity = 3;

    struct Product* p = inventory;
    int total = 0;

    for (int i = 0; i < 3; i++) {
        total += p->price * p->quantity;
        p++; // Pointer arithmetic to move to next struct
    }

    return total; // Should be 50 + 50 + 45 = 145
}