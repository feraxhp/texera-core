//! Single-layer cursor traversing chunks within a RangeSet.

use std::collections::HashSet;
use std::sync::Arc;

use super::utils::cmp_pos_side;
use super::utils::FAR;
use super::LayerCursor;
use super::RangeValue;
use super::RangeSet;
use super::Chunk;


impl<'a, T: RangeValue> LayerCursor<'a, T> {
    /// Creates a cursor for a single layer of a [`RangeSet`].
    pub(crate) fn new(
        layer: &'a RangeSet<T>,
        skip: Option<&'a HashSet<usize>>,
        min_point: isize,
        rank: usize,
    ) -> Self {
        Self {
            layer,
            skip,
            min_point,
            rank,
            from: 0,
            to: 0,
            value: None,
            chunk_index: 0,
            range_index: 0,
        }
    }

    /// Returns the start side bias of the current range.
    #[inline]
    pub(crate) fn start_side(&self) -> i32 {
        self.value.as_ref().map_or(0, |v| v.start_side())
    }

    /// Returns the end side bias of the current range.
    #[inline]
    pub(crate) fn end_side(&self) -> i32 {
        self.value.as_ref().map_or(0, |v| v.end_side())
    }

    fn is_skipped(&self, chunk: &Arc<Chunk<T>>) -> bool {
        self.skip
            .map_or(false, |s| s.contains(&(Arc::as_ptr(chunk) as usize)))
    }

    /// Positions the cursor at `pos` with bias `side`.
    pub(crate) fn goto(&mut self, pos: usize, side: i32) {
        self.chunk_index = 0;
        self.range_index = 0;
        self.goto_inner(pos, side, false);
    }

    fn goto_inner(&mut self, pos: usize, side: i32, mut forward: bool) {
        while self.chunk_index < self.layer.chunk.len() {
            let next = &self.layer.chunk[self.chunk_index];
            if !(self.is_skipped(next)
                || self.layer.chunk_end(self.chunk_index) < pos
                || next.max_point < self.min_point)
            {
                break;
            }
            self.chunk_index += 1;
            forward = false;
        }
        if self.chunk_index < self.layer.chunk.len() {
            let rel_pos = (pos as isize) - (self.layer.chunk_pos[self.chunk_index] as isize);
            let range_index =
                self.layer.chunk[self.chunk_index].find_index(rel_pos, side, true, 0);
            if !forward || self.range_index < range_index {
                self.set_range_index(range_index);
            }
        }
        self.advance();
    }

    /// Moves the cursor forward to `pos` if it is currently before that position.
    pub(crate) fn forward(&mut self, pos: isize, side: i32) {
        if cmp_pos_side(self.to, self.end_side(), pos, side) < 0 {
            self.goto_inner(pos.max(0) as usize, side, true);
        }
    }

    /// Advances the cursor to the next valid range in this layer.
    pub(crate) fn advance(&mut self) {
        loop {
            if self.chunk_index == self.layer.chunk.len() {
                self.from = FAR;
                self.to = FAR;
                self.value = None;
                break;
            } else {
                let chunk_pos = self.layer.chunk_pos[self.chunk_index];
                let chunk = &self.layer.chunk[self.chunk_index];
                let from = chunk_pos + chunk.from[self.range_index];
                let to = chunk_pos + chunk.to[self.range_index];
                let val = chunk.value[self.range_index].clone();
                self.from = from as isize;
                self.to = to as isize;
                let is_pt = val.point();
                self.value = Some(val);
                self.set_range_index(self.range_index + 1);
                if self.min_point < 0 || (is_pt && (self.to - self.from) >= self.min_point) {
                    break;
                }
            }
        }
    }

    fn set_range_index(&mut self, index: usize) {
        if index == self.layer.chunk[self.chunk_index].value.len() {
            self.chunk_index += 1;
            if self.skip.is_some() {
                while self.chunk_index < self.layer.chunk.len()
                    && self.is_skipped(&self.layer.chunk[self.chunk_index])
                {
                    self.chunk_index += 1;
                }
            }
            self.range_index = 0;
        } else {
            self.range_index = index;
        }
    }

    /// Skips to the start of the next chunk in this layer.
    pub(crate) fn next_chunk(&mut self) {
        self.chunk_index += 1;
        self.range_index = 0;
        self.advance();
    }

    /// Compares the current cursor position with another cursor for heap ordering.
    pub(crate) fn compare(&self, other: &Self) -> isize {
        let d1 = cmp_pos_side(self.from, self.start_side(), other.from, other.start_side());
        if d1 != 0 { return d1; }
        let d2 = (self.rank as isize) - (other.rank as isize);
        if d2 != 0 { return d2; }
        cmp_pos_side(self.to, self.end_side(), other.to, other.end_side())
    }
}
