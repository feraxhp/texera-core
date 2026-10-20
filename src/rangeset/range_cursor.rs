use std::collections::HashSet;

use super::utils::cmp_pos_side;
use super::utils::FAR;
use super::RangeSetCursor;
use super::LayerCursor;
use super::RangeValue;
use super::RangeSet;
use super::Range;


fn heap_bubble<T: RangeValue>(heap: &mut [LayerCursor<'_, T>], mut index: usize) {
    loop {
        let mut child_index = (index << 1) + 1;
        if child_index >= heap.len() { break; }
        if child_index + 1 < heap.len()
            && heap[child_index].compare(&heap[child_index + 1]) >= 0
        {
            child_index += 1;
        }
        if heap[index].compare(&heap[child_index]) < 0 { break; }
        heap.swap(index, child_index);
        index = child_index;
    }
}

impl<'a, T: RangeValue> RangeSetCursor<'a, T> {
    pub(crate) fn from_sets(
        sets: &[&'a RangeSet<T>],
        skip: Option<&'a HashSet<usize>>,
        min_point: isize,
    ) -> Self {
        let mut heap = Vec::new();
        for (i, &set) in sets.iter().enumerate() {
            let mut cur_opt = Some(set);
            while let Some(cur) = cur_opt {
                if !cur.is_empty() {
                    if cur.max_point >= min_point {
                        heap.push(LayerCursor::new(cur, skip, min_point, i));
                    }
                    cur_opt = cur.next_layer.as_deref();
                } else {
                    break;
                }
            }
        }
        Self {
            heap,
            from: FAR,
            to: FAR,
            value: None,
            rank: 0,
        }
    }

    /// Returns the start character offset of the current range under the cursor.
    #[inline]
    pub fn from(&self) -> usize { self.from.max(0) as usize }

    /// Returns the end character offset of the current range under the cursor.
    #[inline]
    pub fn to(&self) -> usize { self.to.max(0) as usize }

    #[inline]
    pub(crate) fn start_side(&self) -> i32 {
        self.value.as_ref().map_or(0, |v| v.start_side())
    }

    #[inline]
    pub(crate) fn end_side(&self) -> i32 {
        self.value.as_ref().map_or(0, |v| v.end_side())
    }

    /// Repositions the cursor to the first range ending at or after `pos`.
    pub fn goto(&mut self, pos: usize) {
        self.goto_with_side(pos, -FAR as i32);
    }

    pub(crate) fn goto_with_side(&mut self, pos: usize, side: i32) {
        for cur in &mut self.heap {
            cur.goto(pos, side);
        }
        let len = self.heap.len();
        for i in (0..=(len >> 1)).rev() {
            if i < len {
                heap_bubble(&mut self.heap, i);
            }
        }
        self.advance();
    }

    pub(crate) fn forward(&mut self, pos: isize, side: i32) {
        for cur in &mut self.heap {
            cur.forward(pos, side);
        }
        let len = self.heap.len();
        for i in (0..=(len >> 1)).rev() {
            if i < len {
                heap_bubble(&mut self.heap, i);
            }
        }
        if cmp_pos_side(self.to, self.end_side(), pos, side) < 0 {
            self.advance();
        }
    }

    /// Advances the cursor to the next range in sorted order.
    pub fn advance(&mut self) {
        if self.heap.is_empty() {
            self.from = FAR;
            self.to = FAR;
            self.value = None;
            self.rank = 0;
        } else {
            let top = &mut self.heap[0];
            self.from = top.from;
            self.to = top.to;
            self.value = top.value.clone();
            self.rank = top.rank;
            if top.value.is_some() {
                top.advance();
            }
            heap_bubble(&mut self.heap, 0);
        }
    }
}

impl<'a, T: RangeValue> Iterator for RangeSetCursor<'a, T> {
    type Item = Range<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let val = self.value.clone()?;
        let from = self.from();
        let to = self.to();
        self.advance();
        Some(Range::new(from, to, val))
    }
}
