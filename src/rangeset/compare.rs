use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::changes::ChangeIterator;
use crate::Composable;
use crate::ChangeDesc;
use crate::MapMode;
use crate::Touched;
use crate::Assoc;

use super::utils::same_values;
use super::utils::cmp_val;
use super::utils::FAR;
use super::RangeComparator;
use super::SpanIterator;
use super::RangeValue;
use super::SpanCursor;
use super::RangeSet;


impl<T: RangeValue> RangeSet<T> {
    /// Compares two slices of [`RangeSet`]s (`old_sets` and `new_sets`) linked by a document
    /// `text_diff`, invoking `comparator` for any intervals or points that changed.
    ///
    /// Shared [`Chunk`](crate::Chunk)s that were not touched by `text_diff` are skipped in $O(1)$.
    pub fn compare(
        old_sets: &[&RangeSet<T>],
        new_sets: &[&RangeSet<T>],
        text_diff: &ChangeDesc,
        comparator: &mut impl RangeComparator<T>,
        min_point_size: Option<isize>,
    ) {
        let min_pt = min_point_size.unwrap_or(-1);
        let a: Vec<&RangeSet<T>> = old_sets
            .iter()
            .copied()
            .filter(|set| set.max_point > 0 || (!set.is_empty() && set.max_point >= min_pt))
            .collect();
        let b: Vec<&RangeSet<T>> = new_sets
            .iter()
            .copied()
            .filter(|set| set.max_point > 0 || (!set.is_empty() && set.max_point >= min_pt))
            .collect();
        let shared_chunks = find_shared_chunks(&a, &b, Some(text_diff));

        let mut side_a = SpanCursor::new(&a, Some(&shared_chunks), min_pt);
        let mut side_b = SpanCursor::new(&b, Some(&shared_chunks), min_pt);

        for gap in text_diff.iter_unchanged() {
            compare_cursors(
                &mut side_a,
                gap.map.old,
                &mut side_b,
                gap.map.new,
                gap.length,
                comparator,
            );
        }
        if text_diff.is_empty() && text_diff.len_old() == 0 {
            compare_cursors(&mut side_a, 0, &mut side_b, 0, 0, comparator);
        }
    }

    /// Returns `true` if `old_sets` and `new_sets` produce identical active ranges and point ranges
    /// over `[from..to]`.
    pub fn eq_sets(
        old_sets: &[&RangeSet<T>],
        new_sets: &[&RangeSet<T>],
        from: usize,
        to: Option<usize>,
    ) -> bool {
        let to = to.map(|t| t as isize).unwrap_or(FAR - 1);
        let a: Vec<&RangeSet<T>> = old_sets
            .iter()
            .copied()
            .filter(|set| {
                !set.is_empty() && !new_sets.iter().any(|other| std::ptr::eq(*set, *other))
            })
            .collect();
        let b: Vec<&RangeSet<T>> = new_sets
            .iter()
            .copied()
            .filter(|set| {
                !set.is_empty() && !old_sets.iter().any(|other| std::ptr::eq(*set, *other))
            })
            .collect();
        if a.len() != b.len() { return false; }
        if a.is_empty() { return true; }

        let shared_chunks = find_shared_chunks(&a, &b, None);
        let mut side_a = SpanCursor::new(&a, Some(&shared_chunks), 0);
        side_a.goto(from, -FAR as i32);
        let mut side_b = SpanCursor::new(&b, Some(&shared_chunks), 0);
        side_b.goto(from, -FAR as i32);

        loop {
            if side_a.to != side_b.to
                || !same_values(&side_a.active, &side_b.active)
                || (side_a.point.is_some()
                    && (side_b.point.is_none()
                        || !cmp_val(
                            side_a.point.as_ref().unwrap(),
                            side_b.point.as_ref().unwrap(),
                        )))
            {
                return false;
            }
            if side_a.to > to { return true; }
            side_a.advance();
            side_b.advance();
        }
    }

    /// Iterates over uniform spans and point decorations across multiple [`RangeSet`]s in `[from..to]`,
    /// driving `iterator` ([`SpanIterator`]).
    ///
    /// Returns the number of ranges still open at `to`.
    pub fn spans(
        sets: &[&RangeSet<T>],
        from: usize,
        to: usize,
        iterator: &mut impl SpanIterator<T>,
        min_point_size: Option<isize>,
    ) -> usize {
        let min_pt = min_point_size.unwrap_or(-1);
        let mut cursor = SpanCursor::new(sets, None, min_pt);
        cursor.goto(from, -FAR as i32);
        let mut pos = from;
        let mut open_ranges = cursor.open_start.max(0) as usize;

        loop {
            let cur_to = (cursor.to.max(0) as usize).min(to);
            if let Some(point_val) = cursor.point.clone() {
                let active = cursor.active_for_point(cursor.to);
                let open_count = if cursor.point_from < from {
                    active.len() + 1
                } else if point_val.start_side() < 0 {
                    active.len()
                } else {
                    active.len().min(open_ranges)
                };
                iterator.point(
                    pos,
                    cur_to,
                    &point_val,
                    &active,
                    open_count,
                    cursor.point_rank,
                );
                open_ranges = cursor.open_end(cur_to as isize).min(active.len());
            } else if cur_to > pos {
                iterator.span(pos, cur_to, &cursor.active, open_ranges);
                open_ranges = cursor.open_end(cur_to as isize);
            }
            if cursor.to > (to as isize) {
                return open_ranges
                    + if cursor.point.is_some() && cursor.to > (to as isize) {
                        1
                    } else {
                        0
                    };
            }
            pos = cursor.to.max(0) as usize;
            cursor.advance();
        }
    }
}

fn find_shared_chunks<T: RangeValue>(
    a: &[&RangeSet<T>],
    b: &[&RangeSet<T>],
    text_diff: Option<&ChangeDesc>,
) -> HashSet<usize> {
    let mut in_a: HashMap<usize, usize> = HashMap::new();
    for set in a {
        let mut cur_opt = Some(*set);
        while let Some(cur) = cur_opt {
            for i in 0..cur.chunk.len() {
                if cur.chunk[i].max_point <= 0 {
                    in_a.insert(Arc::as_ptr(&cur.chunk[i]) as usize, cur.chunk_pos[i]);
                }
            }
            cur_opt = cur.next_layer.as_deref();
        }
    }

    let mut shared: HashSet<usize> = HashSet::new();
    for set in b {
        let mut cur_opt = Some(*set);
        while let Some(cur) = cur_opt {
            for i in 0..cur.chunk.len() {
                let ptr = Arc::as_ptr(&cur.chunk[i]) as usize;
                if let Some(&known) = in_a.get(&ptr) {
                    let mapped_known = match text_diff {
                        Some(diff) => diff
                            .map_pos(known, Assoc::Before, MapMode::Simple)
                            .ok()
                            .flatten()
                            .unwrap_or(known),
                        None => known,
                    };
                    let untouched = match text_diff {
                        Some(diff) => {
                            diff.touches_range(known, Some(known + cur.chunk[i].length()))
                                == Touched::No
                        }
                        None => true,
                    };
                    if mapped_known == cur.chunk_pos[i] && untouched {
                        shared.insert(ptr);
                    }
                }
            }
            cur_opt = cur.next_layer.as_deref();
        }
    }
    shared
}

fn compare_cursors<T: RangeValue>(
    a: &mut SpanCursor<'_, T>,
    start_a: usize,
    b: &mut SpanCursor<'_, T>,
    start_b: usize,
    length: usize,
    comparator: &mut impl RangeComparator<T>,
) {
    a.goto(start_a, -FAR as i32);
    b.goto(start_b, -FAR as i32);
    let end_b = (start_b + length) as isize;
    let mut pos = start_b as isize;
    let d_pos = (start_b as isize) - (start_a as isize);
    let bounds = comparator.tracks_bound_change();
    let mut bound_change = false;

    loop {
        let d_end = (a.to + d_pos) - b.to;
        let diff = match d_end != 0 {
            true  => d_end,
            false => (a.end_side as isize) - (b.end_side as isize),
        };
        let end = if diff < 0 { a.to + d_pos } else { b.to };
        let clip_end = end.min(end_b);
        let has_point = a.point.is_some() || b.point.is_some();

        if has_point {
            let same_pt = match (&a.point, &b.point) {
                (Some(pa), Some(pb)) => {
                    cmp_val(pa, pb)
                        && same_values(&a.active_for_point(a.to), &b.active_for_point(b.to))
                }
                _ => false,
            };
            if !same_pt {
                comparator.compare_point(
                    pos.max(0) as usize,
                    clip_end.max(0) as usize,
                    a.point.as_ref(),
                    b.point.as_ref(),
                );
            }
            bound_change = false;
        } else {
            if bound_change {
                comparator.bound_change(pos.max(0) as usize);
            }
            if clip_end > pos && !same_values(&a.active, &b.active) {
                comparator.compare_range(
                    pos.max(0) as usize,
                    clip_end.max(0) as usize,
                    &a.active,
                    &b.active,
                );
            }
            if bounds
                && clip_end < end_b
                && (d_end != 0 || a.open_end(end) != b.open_end(end))
            {
                bound_change = true;
            }
        }

        if end > end_b { break; }
        pos = end;
        if diff <= 0 { a.advance(); }
        if diff >= 0 { b.advance(); }
    }
}
