// Safe C++ test cases for Keller-Verify formal verification

#include <vector>
#include <string>
#include <iostream>

int safe_iterator_range_for() {
    std::vector<int> v = {1, 2, 3, 4, 5};
    int sum = 0;
    for (const auto &x : v) {
        sum += x;
    }
    return sum;
}

int safe_iterator_loop() {
    std::vector<int> v = {10, 20, 30};
    int total = 0;
    for (auto it = v.begin(); it != v.end(); ++it) {
        total += *it;
    }
    return total;
}

void safe_heap_access() {
    int *p = new int(42);
    *p = 100;
    delete p;
}

void safe_double_delete_with_null() {
    int *p = new int(10);
    delete p;
    p = nullptr;
    delete p; // safe in C++
}

int safe_null_check(int *p) {
    if (p != nullptr) {
        return *p;
    }
    return 0;
}

int safe_vector_bounds() {
    std::vector<int> v(10);
    for (size_t i = 0; i < v.size(); i++) {
        v[i] = static_cast<int>(i);
    }
    return v[0] + v[9];
}

int safe_vector_guarded_index(std::vector<int> &v, size_t i) {
    if (i < v.size()) {
        return v[i];
    }
    return -1;
}

int safe_div_guarded(int a, int b) {
    if (b != 0) {
        return a / b;
    }
    return 0;
}

int safe_div_constant(int x) {
    return x / 2;
}

class SafeBase {
public:
    virtual int compute() { return 1; }
    SafeBase() {}
    virtual ~SafeBase() {}
};

class SafeDerived : public SafeBase {
public:
    int compute() override { return 2; }
};

int test_safe_virtual() {
    SafeDerived d;
    return d.compute();
}
