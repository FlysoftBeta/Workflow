#!/usr/bin/env python3
"""Negative controls for the platform extraction's source-equivalence gate."""

import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location(
    "platform_merge", Path(__file__).with_name("check-platform-merge.py"))
check = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check)


def canonical(source):
    return check.canonical("mod runtime {" + source + "}")


class EquivalenceControls(unittest.TestCase):
    def test_scalar_aliases_preserve_width_and_signedness(self):
        before = 'pub type word = ::core::ffi::c_ulong; unsafe extern "C" { fn f(x: word); }'
        after = 'unsafe extern "C" { fn f(_: u64); }'
        self.assertEqual(canonical(before), canonical(after))
        self.assertNotEqual(canonical(before), canonical(after.replace("u64", "u32")))
        self.assertNotEqual(canonical(before), canonical(after.replace("u64", "i64")))

    def test_external_alias_is_not_a_local_alias(self):
        source = 'pub type intptr_t = i64; fn f(x: ::libc::intptr_t) {}'
        self.assertEqual(canonical(source), canonical('fn f(x: ::libc::intptr_t) {}'))

    def test_character_abi_and_layout_are_not_erased(self):
        source = '#[repr(C)] pub struct S { pub a: u32, pub b: ::core::ffi::c_char }'
        self.assertNotEqual(canonical(source), canonical(source.replace("::core::ffi::c_char", "i8")))
        self.assertNotEqual(canonical(source), canonical(source.replace("a: u32, pub b: ::core::ffi::c_char", "b: ::core::ffi::c_char, pub a: u32")))

    def test_item_order_is_ignored_but_execution_order_is_not(self):
        a, b = 'fn a() {}', 'fn b() {}'
        self.assertEqual(canonical(a + b), canonical(b + a))
        self.assertNotEqual(canonical('fn f() { a(); b(); }'), canonical('fn f() { b(); a(); }'))

    def test_strings_and_result_values_are_compared(self):
        source = 'fn f() -> i32 { let s = b"size_t two words"; return 42; }'
        self.assertNotEqual(canonical(source), canonical(source.replace("two words", "twowords")))
        self.assertNotEqual(canonical(source), canonical(source.replace("42", "41")))

    def test_mode_constants_are_compared_by_actual_value(self):
        before = 'fn f(x: u32) -> bool { x == 0o120000 as u32 }'
        after = 'pub const S_IFLNK: i32 = 0o120000 as i32; fn f(x: u32) -> bool { x == S_IFLNK as u32 }'
        self.assertEqual(canonical(before), canonical(after))
        self.assertNotEqual(canonical(before), canonical(after.replace("0o120000", "0o100000")))

    def test_empty_constructor_fields_are_still_compared(self):
        before = 'fn f() { let s: stat = stat { a: 0, b: [0; 3] }; }'
        after = 'unsafe fn platform_empty_stat() -> stat { stat { a: 0, b: [0; 3] } } fn f() { let s: stat = platform_empty_stat(); }'
        self.assertEqual(canonical(before), canonical(after))
        self.assertNotEqual(canonical(before), canonical(after.replace("a: 0", "a: 1")))
        with self.assertRaises(AssertionError):
            canonical(after.replace("a: 0", "a: side_effect()"))

    def test_only_assignment_blocks_can_lose_scope(self):
        self.assertEqual(canonical('fn f() { x = y; }'), canonical('fn f() { { x = y; } }'))
        self.assertNotEqual(canonical('fn f() { let x = y; }'), canonical('fn f() { { let x = y; } }'))

    def test_scalar_helper_does_not_hide_changed_predicates(self):
        before = 'fn f(status: i32) { if status == 0 { end(); } }'
        after = 'fn platform_expr_wait(status: i32) -> bool { status == 0 } fn f(status: i32) { if platform_expr_wait(status) { end(); } }'
        self.assertEqual(canonical(before), canonical(after))
        self.assertNotEqual(canonical(before), canonical(after.replace("status == 0", "status != 0")))

    def test_errno_retains_its_linked_symbol(self):
        before = 'unsafe extern "C" { fn __errno() -> *mut i32; } fn f() { *__errno() = 0; }'
        after = 'unsafe extern "C" { #[link_name = "__errno"] fn errno() -> *mut i32; } fn f() { *errno() = 0; }'
        self.assertEqual(canonical(before), canonical(after))
        self.assertNotEqual(canonical(before), canonical(after.replace('"__errno"', '"__errno_location"')))

    def test_compiler_selects_all_three_target_branches(self):
        with tempfile.TemporaryDirectory() as directory:
            scratch = Path(directory)
            source = scratch / "selected.rs"
            source.write_text('''
#[cfg(target_os = "linux")] fn selected() -> i32 { 1 }
#[cfg(all(target_os = "android", target_arch = "x86_64"))] fn selected() -> i32 { 2 }
#[cfg(target_arch = "aarch64")] fn selected() -> i32 { 3 }
''')
            for value, target in enumerate(check.TARGETS.values(), 1):
                selected = check.canonical(check.expand(source, target, scratch))
                self.assertEqual(canonical(f'fn selected() -> i32 {{ {value} }}'), selected)


if __name__ == "__main__":
    unittest.main()
