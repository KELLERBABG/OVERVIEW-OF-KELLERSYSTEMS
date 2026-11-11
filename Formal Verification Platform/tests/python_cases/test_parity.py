"""Comprehensive test fixture for Keller Python parity upgrades."""
from typing import Optional
from collections import defaultdict

def loop_range_len(items: list):
    res = 0
    for i in range(len(items)):
        res += items[i]
    return res

def loop_range_bounds(items: list):
    res = 0
    for i in range(0, 5):
        res += items[i]
    return res

def loop_enumerate(items: list):
    res = 0
    for idx, val in enumerate(items):
        res += items[idx]
    return res

def loop_while_bound(items: list, n: int):
    i = 0
    while i < len(items):
        res = items[i]
        i += 1

def handled_div_zero(a: int, b: int) -> float:
    try:
        return a / b
    except ZeroDivisionError:
        return 0.0

def handled_multi_except(items: list, idx: int):
    try:
        return items[idx]
    except (IndexError, KeyError):
        return None

def handled_catch_all(x: int, y: int):
    try:
        return x / y
    except Exception:
        return 0

def unhandled_div_in_handler(a: int, b: int):
    try:
        return a + 1
    except ValueError:
        return a / b

def helper_precond(a: int, b: int) -> float:
    assert b > 0
    return a / b

def caller_good():
    return helper_precond(10, 2)

def caller_branch(x: int):
    if x > 0:
        return helper_precond(100, x)
    return 0.0

# ----------------------------------------------------------------------------
# Parity Tests for None Dereference (AttributeError)
# ----------------------------------------------------------------------------

def guarded_none_check(user: Optional[str]):
    if user is not None:
        return user.strip()
    return ""

def assert_guarded_none_check(config: Optional[dict]):
    assert config is not None
    return config.keys()

def buggy_none_deref(item: Optional[object]):
    return item.value

def handled_none_deref(item: Optional[object]):
    try:
        return item.value
    except AttributeError:
        return "fallback"

# ----------------------------------------------------------------------------
# Parity Tests for Key Error on Dictionaries (KeyError)
# ----------------------------------------------------------------------------

def guarded_key_lookup(cfg: dict, key: str):
    if key in cfg:
        return cfg[key]
    return "default"

def assigned_key_lookup(d: dict):
    d["port"] = 8080
    return d["port"]

def buggy_key_lookup(cfg: dict, key: str):
    return cfg[key]

def handled_key_lookup(cfg: dict, key: str):
    try:
        return cfg[key]
    except KeyError:
        return None

# ----------------------------------------------------------------------------
# Parity Tests for Type Errors (TypeError)
# ----------------------------------------------------------------------------

def buggy_len_on_scalar(val: int):
    return len(val)

def handled_type_error(val: int):
    try:
        return len(val)
    except TypeError:
        return 0

def buggy_str_subtraction(s: str, n: int):
    return s - n

# ----------------------------------------------------------------------------
# Group A Hardening: Advanced Python Semantics
# ----------------------------------------------------------------------------

def test_negative_index_safe(items: list):
    assert len(items) >= 1
    return items[-1]

def test_slice_never_crashes(items: list):
    # Slices in Python clamp rather than raising IndexError
    return items[100:200]

def test_defaultdict_safe(d: defaultdict):
    # defaultdict never raises KeyError
    return d["any_missing_key"]

def test_dict_pop_buggy(d: dict, k: str):
    # pop without default raises KeyError if k not in d
    return d.pop(k)

def test_ternary_none_safe(val: Optional[str]):
    # Ternary context passes guard into true branch
    return val.strip() if val else ""

def test_ternary_div_safe(x: int):
    # Ternary context passes guard into true branch
    return 100 / x if x != 0 else 0

def test_scalar_subscript_buggy(x: int):
    # Subscripting scalar is TypeError
    return x[0]

# ----------------------------------------------------------------------------
# Group B & C Hardening: While Loops, Reflection, Iterators & Eval
# ----------------------------------------------------------------------------

def test_while_symmetric_bound(items: list):
    # B4: Symmetric comparison (len(items) > i) with initial i = 0
    i = 0
    while len(items) > i:
        val = items[i]
        i += 1
    return i

def test_getattr_reflection_buggy(item: Optional[object]):
    # C1: getattr without default dereferences item (AttributeError if None)
    return getattr(item, "secret")

def test_getattr_reflection_safe(item: Optional[object]):
    # C1: getattr with default value does not crash on None
    return getattr(item, "secret", "default_val")

def test_setattr_reflection_buggy(item: Optional[object]):
    # C1: setattr on None always raises AttributeError
    setattr(item, "prop", 42)

def test_next_iterator_exhaust_buggy(it: object):
    # C3: next(it) without default raises StopIteration if exhausted
    return next(it)

def test_next_iterator_handled(it: object):
    # C3: handled StopIteration
    try:
        return next(it)
    except StopIteration:
        return None

def test_next_iterator_default_safe(it: object):
    # C3: next(it, default) never raises StopIteration
    return next(it, "exhausted_fallback")

def test_eval_code_buggy(user_code: str):
    # C2: unconstrained dynamic code execution
    return eval(user_code)

def test_eval_code_handled(user_code: str):
    # C2: handled dynamic code evaluation
    try:
        return eval(user_code)
    except SyntaxError:
        return 0

# ----------------------------------------------------------------------------
# P1 & P2: Unpacking Mismatches & Format String Errors
# ----------------------------------------------------------------------------

def test_static_unpacking_mismatch():
    # P1: Unpacking mismatch (ValueError: not enough values to unpack)
    a, b = [1]
    return a + b

def test_dynamic_unpacking_mismatch(items: list):
    # P1: Dynamic sequence length mismatch
    a, b = items
    return a + b

def test_unpacking_handled(items: list):
    # P1: Handled ValueError
    try:
        a, b = items
        return a + b
    except ValueError:
        return 0

def test_modulo_format_mismatch():
    # P2: Modulo formatting mismatch (TypeError: not enough arguments for format string)
    template = "%s: %d"
    return template % ("age",)

def test_modulo_format_handled():
    # P2: Handled format mismatch
    try:
        return "%s: %d" % ("age",)
    except TypeError:
        return ""

def test_fstring_format_mismatch(label: str):
    # P2: f-string with string formatted as integer (ValueError: Unknown format code 'd' for object of type 'str')
    return f"{label:d}"

# ----------------------------------------------------------------------------
# C4: Concurrency & Threading Races
# ----------------------------------------------------------------------------

import threading

def test_unlocked_thread_spawn():
    # C4: Unlocked thread spawning flags concurrency race risk
    t = threading.Thread(target=lambda: None)
    return t

def test_locked_thread_spawn():
    # C4: Thread spawned within lock scope is safe
    lock = threading.Lock()
    with lock:
        t = threading.Thread(target=lambda: None)
        return t

def test_handled_thread_spawn():
    # C4: Handled RuntimeError
    try:
        return threading.Thread(target=lambda: None)
    except RuntimeError:
        return None
