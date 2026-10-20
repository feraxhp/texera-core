use std::sync::Arc;

use crate::ChangeDesc;
use crate::MapMode;

use super::utils::cmp_pos_side;
use super::utils::side_to_assoc;
use super::utils::FAR;
use super::RangeValue;
use super::Chunk;


impl<T: RangeValue> Chunk<T> {
    /// Creates a new B-tree chunk from relative offsets and values.
    ///
    /// - `from`: Relative start character offsets from the chunk base.
    /// - `to`: Relative end character offsets from the chunk base.
    /// - `value`: Associated [`RangeValue`] items for each interval.
    /// - `max_point`: Maximum size of any point decoration contained in this chunk (`-1` if none).
    pub fn new(from: Vec<usize>, to: Vec<usize>, value: Vec<T>, max_point: isize) -> Self {
        Self { from, to, value, max_point }
    }

    /// Returns the end offset of the furthest range in this chunk relative to its base.
    #[inline]
    pub fn length(&self) -> usize {
        self.to.last().copied().unwrap_or(0)
    }

    /// Binary-searches for the index within this chunk matching `(pos, side)`.
    ///
    /// When `end` is `true`, searches along the `to` array using [`RangeValue::end_side`].
    /// When `end` is `false`, searches along the `from` array using [`RangeValue::start_side`].
    /// Starts the search at index `start_at`.
    pub fn find_index(&self, pos: isize, side: i32, end: bool, start_at: usize) -> usize {
        let arr = if end { &self.to } else { &self.from };
        let mut lo = start_at;
        let mut hi = arr.len();

        loop {
            if lo == hi { return lo; }
            let mid = (lo + hi) >> 1;
            let mid_side = match end {
                true  => self.value[mid].end_side(),
                false => self.value[mid].start_side(),
            };
            let diff = cmp_pos_side(arr[mid] as isize, mid_side, pos, side);
            if mid == lo {
                return if diff >= 0 { lo } else { hi };
            }
            if diff >= 0 {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
    }

    pub(crate) fn between<F>(&self, offset: usize, from: isize, to: isize, f: &mut F) -> bool
    where
        F: FnMut(usize, usize, &T) -> bool,
    {
        let start = self.find_index(from, -FAR as i32, true, 0);
        let end = self.find_index(to, FAR as i32, false, start);
        for i in start..end {
            if !f(self.from[i] + offset, self.to[i] + offset, &self.value[i]) {
                return false;
            }
        }
        true
    }

    pub(crate) fn map(&self, offset: usize, changes: &ChangeDesc) -> Option<(usize, Arc<Chunk<T>>)> {
        let mut value = Vec::new();
        let mut from = Vec::new();
        let mut to = Vec::new();
        let mut new_pos: Option<usize> = None;
        let mut max_point: isize = -1;

        for i in 0..self.value.len() {
            let val = &self.value[i];
            let cur_from = self.from[i] + offset;
            let cur_to = self.to[i] + offset;
            let (new_from, new_to);

            if cur_from == cur_to {
                let mapped = changes
                    .map_pos(cur_from, side_to_assoc(val.start_side()), val.map_mode())
                    .ok()
                    .flatten();
                let Some(mapped) = mapped else { continue; };
                new_from = mapped;
                if val.start_side() != val.end_side() {
                    let mapped_to = changes
                        .map_pos(cur_from, side_to_assoc(val.end_side()), MapMode::Simple)
                        .ok()
                        .flatten()
                        .unwrap_or(mapped);
                    if mapped_to < new_from { continue; }
                    new_to = mapped_to;
                } else {
                    new_to = mapped;
                }
            } else {
                new_from = changes
                    .map_pos(cur_from, side_to_assoc(val.start_side()), MapMode::Simple)
                    .ok()
                    .flatten()
                    .unwrap_or(0);
                new_to = changes
                    .map_pos(cur_to, side_to_assoc(val.end_side()), MapMode::Simple)
                    .ok()
                    .flatten()
                    .unwrap_or(0);
                if new_from > new_to
                    || (new_from == new_to && val.start_side() > 0 && val.end_side() <= 0)
                {
                    continue;
                }
            }

            let len_diff = (new_to as isize) - (new_from as isize);
            let side_diff = (val.end_side() as isize) - (val.start_side() as isize);
            if (if len_diff != 0 { len_diff } else { side_diff }) < 0 {
                continue;
            }

            let base = *new_pos.get_or_insert(new_from);
            if val.point() {
                max_point = max_point.max((new_to - new_from) as isize);
            }
            value.push(val.clone());
            from.push(new_from - base);
            to.push(new_to - base);
        }

        new_pos.map(|pos| (pos, Arc::new(Chunk::new(from, to, value, max_point))))
    }
}
