// Vulnerable C++ test cases for Keller-Verify formal verification

#include <vector>
#include <string>

// 1. cpp-iterator-invalidation
void test_iterator_invalidation_range_for() {
    std::vector<int> v = {1, 2, 3};
    for (auto &x : v) {
        v.push_back(x); // cpp-iterator-invalidation
    }
}

void test_iterator_invalidation_sequential() {
    std::vector<int> v = {1, 2, 3};
    auto it = v.begin();
    v.push_back(10);
    *it = 20; // cpp-iterator-invalidation
}

// 2. cpp-pure-virtual-call
class AbstractBase {
public:
    virtual void pure_method() = 0;
    AbstractBase() {
        pure_method(); // cpp-pure-virtual-call
    }
    virtual ~AbstractBase() {}
};

class AbstractDestruct {
public:
    virtual void pure_method() = 0;
    AbstractDestruct() {}
    virtual ~AbstractDestruct() {
        pure_method(); // cpp-pure-virtual-call
    }
};

// 3. cpp-use-after-delete
void test_use_after_delete() {
    int *p = new int(10);
    delete p;
    *p = 20; // cpp-use-after-delete
}

void test_use_after_delete_array() {
    int *arr = new int[5];
    delete[] arr;
    arr[0] = 1; // cpp-use-after-delete
}

// 4. cpp-double-delete
void test_double_delete() {
    int *p = new int(42);
    delete p;
    delete p; // cpp-double-delete
}

// 5. cpp-null-deref
void test_null_deref() {
    int *p = nullptr;
    *p = 100; // cpp-null-deref
}

// 6. cpp-vector-oob
void test_vector_oob_constant() {
    std::vector<int> v(5);
    v[5] = 99; // cpp-vector-oob
    v[-1] = 1; // cpp-vector-oob
}

void test_vector_oob_loop() {
    std::vector<int> v = {1, 2, 3};
    for (size_t i = 0; i <= v.size(); i++) {
        v[i] = static_cast<int>(i); // cpp-vector-oob
    }
}

// 7. cpp-div-by-zero
void test_div_by_zero_constant() {
    int x = 10 / 0; // cpp-div-by-zero
}

void test_div_by_zero_variable() {
    int y = 0;
    int z = 100 / y; // cpp-div-by-zero
}
