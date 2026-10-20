use std::sync::Arc;

use mynox::Error;

use crate::errors::range_error;

use super::utils::cmp_pos_side;
use super::utils::CHUNK_SIZE;
use super::utils::FAR;
use super::RangeSetBuilder;
use super::RangeValue;
use super::RangeSet;
use super::Chunk;


impl<T: RangeValue> Default for RangeSetBuilder<T> {
    fn default() -> Self { Self::new() }
}

impl<T: RangeValue> RangeSetBuilder<T> {
    /// Creates a new, empty [`RangeSetBuilder`].
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            chunk_pos: Vec::new(),
            chunk_start: -1,
            last: None,
            last_from: -FAR,
            last_to: -FAR,
            from: Vec::new(),
            to: Vec::new(),
            value: Vec::new(),
            max_point: -1,
            set_max_point: -1,
            next_layer: None,
        }
    }

    fn finish_chunk(&mut self, new_arrays: bool) {
        let from = match new_arrays {
            true  => std::mem::take(&mut self.from),
            false => self.from.clone(),
        };
        let to = match new_arrays {
            true  => std::mem::take(&mut self.to),
            false => self.to.clone(),
        };
        let value = match new_arrays {
            true  => std::mem::take(&mut self.value),
            false => self.value.clone(),
        };
        self.chunks.push(Arc::new(Chunk::new(from, to, value, self.max_point)));
        self.chunk_pos.push(self.chunk_start as usize);
        self.chunk_start = -1;
        self.set_max_point = self.set_max_point.max(self.max_point);
        self.max_point = -1;
    }

    /// Adds a range `[from..to]` with `value` to the builder.
    ///
    /// # Panics
    ///
    /// Panics if `from > to` or if the range starts before the previous range added to this builder.
    pub fn add(&mut self, from: usize, to: usize, value: T) {
        self.try_add(from, to, value)
            .expect("Ranges must be added sorted by `from` position and `start_side`");
    }

    /// Fallible version of [`RangeSetBuilder::add`] that returns a [`RangeError`](crate::ErrorTypes::RangeError)
    /// if `from > to` or if ranges are not added in sorted `(from, start_side)` order.
    pub fn try_add(&mut self, from: usize, to: usize, value: T) -> Result<(), Error> {
        if !self.add_inner(from, to, value.clone())? {
            self.next_layer
                .get_or_insert_with(|| Box::new(RangeSetBuilder::new()))
                .try_add(from, to, value)?;
        }
        Ok(())
    }

    pub(crate) fn add_inner(&mut self, from: usize, to: usize, value: T) -> Result<bool, Error> {
        if from > to {
            return Err(range_error("Range `from` cannot be greater than `to`"));
        }
        let from_i = from as isize;
        let to_i = to as isize;
        let last_end_side = self.last.as_ref().map_or(0, |l| l.end_side());
        let last_start_side = self.last.as_ref().map_or(0, |l| l.start_side());

        let diff = cmp_pos_side(from_i, value.start_side(), self.last_to, last_end_side);
        if diff <= 0
            && cmp_pos_side(from_i, value.start_side(), self.last_from, last_start_side) < 0
        {
            return Err(range_error(
                "Ranges must be added sorted by `from` position and `start_side`",
            ));
        }
        if diff < 0 {
            return Ok(false);
        }
        if self.from.len() == CHUNK_SIZE {
            self.finish_chunk(true);
        }
        if self.chunk_start < 0 {
            self.chunk_start = from_i;
        }
        let base = self.chunk_start as usize;
        self.from.push(from - base);
        self.to.push(to - base);
        if value.point() {
            self.max_point = self.max_point.max(to_i - from_i);
        }
        self.last = Some(value.clone());
        self.last_from = from_i;
        self.last_to = to_i;
        self.value.push(value);
        Ok(true)
    }

    pub(crate) fn add_chunk(&mut self, from: usize, chunk: Arc<Chunk<T>>) -> bool {
        let from_i = from as isize;
        let last_end_side = self.last.as_ref().map_or(0, |l| l.end_side());
        if cmp_pos_side(
            from_i,
            chunk.value[0].start_side(),
            self.last_to,
            last_end_side,
        ) < 0
        {
            return false;
        }
        if !self.from.is_empty() {
            self.finish_chunk(true);
        }
        self.set_max_point = self.set_max_point.max(chunk.max_point);
        let last = chunk.value.len() - 1;
        self.last = Some(chunk.value[last].clone());
        self.last_from = (chunk.from[last] + from) as isize;
        self.last_to = (chunk.to[last] + from) as isize;
        self.chunks.push(chunk);
        self.chunk_pos.push(from);
        true
    }

    /// Finalizes the builder and returns the constructed [`RangeSet`].
    pub fn finish(self) -> RangeSet<T> {
        self.finish_inner(RangeSet::empty())
    }

    pub(crate) fn finish_inner(mut self, next: RangeSet<T>) -> RangeSet<T> {
        if !self.from.is_empty() {
            self.finish_chunk(true);
        }
        if self.chunks.is_empty() {
            return next;
        }
        let next_set = match self.next_layer.take() {
            Some(nl) => nl.finish_inner(next),
            None => next,
        };
        let next_arc = match next_set.is_empty() {
            true  => None,
            false => Some(Arc::new(next_set)),
        };
        RangeSet::create(self.chunk_pos, self.chunks, next_arc, self.set_max_point)
    }
}
