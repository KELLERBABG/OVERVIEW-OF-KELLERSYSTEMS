# ============================================================================
#  Keller-Verify Generated pytest Reproduction Suite
#  Generated from SMT-LIB 2.6 formal verification counterexample models
# ============================================================================

import pytest


def test_keller_loop_range_bounds_line_14():
    """Formal verification counterexample for loop_range_bounds (line 14)
    Defect class: python-index-out-of-bounds
    Witness: [i=0, len_items=0, res=0, res_is_none=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(IndexError):
        # Call target under witness parameters
        # loop_range_bounds(i=0, len_items=0, res=0, res_is_none=0)
        pass

def test_keller_unhandled_div_in_handler_line_51():
    """Formal verification counterexample for unhandled_div_in_handler (line 51)
    Defect class: python-zero-division
    Witness: [b=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(ZeroDivisionError):
        # Call target under witness parameters
        # unhandled_div_in_handler(b=0)
        pass

def test_keller_assert_guarded_none_check_line_75():
    """Formal verification counterexample for assert_guarded_none_check (line 75)
    Defect class: python-assertion-failure
    Witness: [config_is_none=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # assert_guarded_none_check(config_is_none=1)
        pass

def test_keller_buggy_none_deref_line_79():
    """Formal verification counterexample for buggy_none_deref (line 79)
    Defect class: python-attribute-error
    Witness: [item_is_none=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AttributeError):
        # Call target under witness parameters
        # buggy_none_deref(item_is_none=1)
        pass

def test_keller_buggy_key_lookup_line_101():
    """Formal verification counterexample for buggy_key_lookup (line 101)
    Defect class: python-key-error
    Witness: [key_in_cfg=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(KeyError):
        # Call target under witness parameters
        # buggy_key_lookup(key_in_cfg=0)
        pass

def test_keller_buggy_len_on_scalar_line_114():
    """Formal verification counterexample for buggy_len_on_scalar (line 114)
    Defect class: python-type-error
    Witness: [type_mismatch=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(TypeError):
        # Call target under witness parameters
        # buggy_len_on_scalar(type_mismatch=1)
        pass

def test_keller_buggy_str_subtraction_line_123():
    """Formal verification counterexample for buggy_str_subtraction (line 123)
    Defect class: python-type-error
    Witness: [type_mismatch=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(TypeError):
        # Call target under witness parameters
        # buggy_str_subtraction(type_mismatch=1)
        pass

def test_keller_test_negative_index_safe_line_130():
    """Formal verification counterexample for test_negative_index_safe (line 130)
    Defect class: python-assertion-failure
    Witness: [len_items=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(AssertionError):
        # Call target under witness parameters
        # test_negative_index_safe(len_items=0)
        pass

def test_keller_test_dict_pop_buggy_line_143():
    """Formal verification counterexample for test_dict_pop_buggy (line 143)
    Defect class: python-key-error
    Witness: [k_in_d=0]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(KeyError):
        # Call target under witness parameters
        # test_dict_pop_buggy(k_in_d=0)
        pass

def test_keller_test_scalar_subscript_buggy_line_155():
    """Formal verification counterexample for test_scalar_subscript_buggy (line 155)
    Defect class: python-type-error
    Witness: [type_mismatch=1]"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises(TypeError):
        # Call target under witness parameters
        # test_scalar_subscript_buggy(type_mismatch=1)
        pass
