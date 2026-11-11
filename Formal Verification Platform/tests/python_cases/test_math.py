"""Test fixtures for Keller Python verification."""

def safe_div(a: int, b: int) -> float:
    if b > 0:
        return a / b
    return 0.0

def buggy_div(a: int, b: int) -> float:
    return a / b

def guarded_div_with_precond(a: int, b: int) -> float:
    assert b != 0, "denominator cannot be zero"
    return a / b

def caller_guarded_div():
    return guarded_div_with_precond(10, 2)

def buggy_modulo(x: int, m: int) -> int:
    return x % m

def safe_index(items: list, idx: int):
    if 0 <= idx and idx < len(items):
        return items[idx]
    return None

def buggy_index(items: list, idx: int):
    return items[idx]

def loop_index(items: list):
    res = 0
    for i in range(len(items)):
        res += items[i]
    return res

def safe_assert(x: int):
    if x > 10:
        assert x > 0

def buggy_assert(x: int):
    assert x > 0

def handled_zero_division(a: int, b: int) -> float:
    try:
        return a / b
    except ZeroDivisionError:
        return 0.0

def handled_index_bounds(items: list, idx: int):
    try:
        return items[idx]
    except IndexError:
        return None
