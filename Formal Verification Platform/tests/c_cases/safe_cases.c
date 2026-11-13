// Safe C test cases for C-RMM formal verification

#include <stdlib.h>
#include <stdio.h>

void safe_array_access(void) {
    int arr[10];
    for (int i = 0; i < 10; i++) {
        arr[i] = i * 2;
    }
}

void safe_heap_access(void) {
    int *p = (int *)malloc(5 * sizeof(int));
    if (p != NULL) {
        p[0] = 42;
        p[4] = 99;
        free(p);
    }
}

int safe_division(int a, int b) {
    if (b != 0) {
        return a / b;
    }
    return 0;
}

int safe_constant_div(int x) {
    return x / 2;
}

int safe_arithmetic(int a) {
    int x = 100;
    int y = 200;
    return x + y;
}
