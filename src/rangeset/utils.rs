use crate::Assoc;

use super::RangeValue;
use super::Range;


pub(crate) const CHUNK_SIZE: usize = 250;
pub(crate) const FAR: isize = 1_000_000_000;

#[inline]
pub(crate) fn side_to_assoc(side: i32) -> Assoc {
    if side < 0 { Assoc::Before } else { Assoc::After }
}

#[inline]
pub(crate) fn cmp_pos_side(pos_a: isize, side_a: i32, pos_b: isize, side_b: i32) -> isize {
    let diff = pos_a - pos_b;
    if diff != 0 {
        diff
    } else {
        (side_a as isize) - (side_b as isize)
    }
}

pub(crate) fn cmp_range<T: RangeValue>(a: &Range<T>, b: &Range<T>) -> std::cmp::Ordering {
    a.from
        .cmp(&b.from)
        .then_with(|| a.value.start_side().cmp(&b.value.start_side()))
}

#[inline]
pub(crate) fn cmp_val<T: RangeValue>(a: &T, b: &T) -> bool {
    std::ptr::eq(a, b) || a.eq(b)
}

pub(crate) fn same_values<T: RangeValue>(a: &[T], b: &[T]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    for i in 0..a.len() {
        if !cmp_val(&a[i], &b[i]) {
            return false;
        }
    }
    true
}
