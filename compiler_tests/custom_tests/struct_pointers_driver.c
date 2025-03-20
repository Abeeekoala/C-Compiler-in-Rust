// struct_pointers_driver.c
int x();

int main() {
    // Expected result: area = 5*10 = 50, perimeter = 2*(5+10) = 30
    // Total: 50 + 30 = 80
    return !(x() == 80);
}