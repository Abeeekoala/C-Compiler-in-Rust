// nested_structs.c
struct Date {
    int day;
    int month;
    int year;
};

struct Person {
    int id;
    struct Date birthdate;
};

int calculate_age(struct Person* p, int current_year) {
    return current_year - p->birthdate.year;
}

int x() {
    struct Person person;
    person.id = 12345;
    person.birthdate.day = 15;
    person.birthdate.month = 5;
    person.birthdate.year = 1990;

    int age = calculate_age(&person, 2023);
    return person.id % 100 + age; // Should be 45 + 33 = 78
}