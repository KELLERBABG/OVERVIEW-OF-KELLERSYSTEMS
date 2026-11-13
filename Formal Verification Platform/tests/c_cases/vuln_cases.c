// Vulnerable C test cases for C-RMM formal verification

#include <stdlib.h>

void test_buffer_overflow(void) {
    int arr[10];
    arr[10] = 42; // c-buffer-overflow: bound is 10, index 10 is OOB
    arr[-1] = 7;  // c-buffer-overflow: negative index
}

void test_buffer_overflow_loop(void) {
    int arr[10];
    for (int i = 0; i <= 10; i++) {
        arr[i] = i; // c-buffer-overflow: reaches 10
    }
}

void test_use_after_free(void) {
    int *p = (int *)malloc(10 * sizeof(int));
    free(p);
    *p = 100; // c-use-after-free
}

void test_double_free(void) {
    int *p = (int *)malloc(10 * sizeof(int));
    free(p);
    free(p); // c-double-free
}

void test_null_deref(void) {
    int *p = NULL;
    *p = 50; // c-null-deref
}

void test_div_by_zero(void) {
    int x = 10 / 0; // c-div-by-zero
    int y = 0;
    int z = 20 / y; // c-div-by-zero
}

void test_signed_overflow(void) {
    int a = 2147483647 + 1; // c-signed-overflow
}
