use std::sync::Arc;

use mynox::Error;

use crate::Composable;
use crate::ChangeDesc;
use crate::MapMode;
use crate::Touched;
use crate::Assoc;

use super::utils::cmp_pos_side;
use super::utils::cmp_range;
use super::utils::FAR;
use super::RangeSetBuilder;
use super::RangeSetCursor;
use super::RangeSetUpdate;
use super::LayerCursor;
use super::RangeValue;
use super::RangeSet;
use super::Chunk;
use super::Range;


impl<T: RangeValue> Default for RangeSet<T> {
    fn default() -> Self { Self::empty() }
}

impl<T: RangeValue> PartialEq for RangeSet<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.is_empty() && other.is_empty() {
            return true;
        }
        if self.is_empty() != other.is_empty() {
            return false;
        }
        Self::eq_sets(&[self], &[other], 0, None)
    }
}

impl<T: RangeValue> Eq for RangeSet<T> {}

impl<T: RangeValue> RangeSet<T> {
    /// Creates an empty [`RangeSet`].
    pub fn empty() -> Self {
        Self {
            chunk_pos: Vec::new(),
            chunk: Vec::new(),
            next_layer: None,
            max_point: -1,
        }
    }

    pub(crate) fn create(
        chunk_pos: Vec<usize>,
        chunk: Vec<Arc<Chunk<T>>>,
        next_layer: Option<Arc<RangeSet<T>>>,
        max_point: isize,
    ) -> Self {
        let next_layer = next_layer.filter(|n| !n.is_empty());
        Self { chunk_pos, chunk, next_layer, max_point }
    }

    /// Returns `true` if this set contains no ranges.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.chunk.is_empty() && self.next_layer.is_none()
    }

    /// Returns the end offset of the furthest range in this set (`0` if empty).
    pub fn length(&self) -> usize {
        if self.chunk.is_empty() {
            0
        } else {
            let last_end = self.chunk_end(self.chunk.len() - 1);
            let next_len = self.next_layer.as_ref().map_or(0, |n| n.length());
            last_end.max(next_len)
        }
    }

    /// Returns the total number of [`Range`]s stored across all chunks and layers in this set.
    pub fn size(&self) -> usize {
        if self.is_empty() { return 0; }
        let mut size = self.next_layer.as_ref().map_or(0, |n| n.size());
        for chunk in &self.chunk {
            size += chunk.value.len();
        }
        size
    }

    #[inline]
    pub(crate) fn chunk_end(&self, index: usize) -> usize {
        self.chunk_pos[index] + self.chunk[index].length()
    }

    /// Constructs a [`RangeSet`] from an iterator of [`Range`]s.
    ///
    /// If `sort` is `true`, ranges are sorted before insertion; if `sort` is `false`, ranges must
    /// already be ordered by `(from, start_side)` or this method will panic.
    pub fn of(ranges: impl IntoIterator<Item = Range<T>>, sort: bool) -> Self {
        Self::try_of(ranges, sort)
            .expect("Ranges must be added sorted by `from` position and `start_side`")
    }

    /// Fallible version of [`RangeSet::of`] that returns a [`RangeError`](crate::ErrorTypes::RangeError)
    /// if `sort == false` and the input ranges are not sorted.
    pub fn try_of(ranges: impl IntoIterator<Item = Range<T>>, sort: bool) -> Result<Self, Error> {
        let mut vec: Vec<Range<T>> = ranges.into_iter().collect();
        if sort {
            let mut needs_sort = false;
            for i in 1..vec.len() {
                if cmp_range(&vec[i - 1], &vec[i]) == std::cmp::Ordering::Greater {
                    needs_sort = true;
                    break;
                }
            }
            if needs_sort {
                vec.sort_by(cmp_range);
            }
        }
        let mut builder = RangeSetBuilder::new();
        for range in vec {
            builder.try_add(range.from, range.to, range.value)?;
        }
        Ok(builder.finish())
    }

    /// Combines a slice of [`RangeSet`]s into a single layered [`RangeSet`].
    pub fn join(sets: &[RangeSet<T>]) -> Self {
        if sets.is_empty() { return Self::empty(); }
        let mut result = sets[sets.len() - 1].clone();
        for i in (0..sets.len() - 1).rev() {
            let mut layers = Vec::new();
            let mut cur = Some(&sets[i]);
            while let Some(layer) = cur {
                if !layer.is_empty() {
                    layers.push(layer);
                    cur = layer.next_layer.as_deref();
                } else {
                    break;
                }
            }
            for layer in layers {
                let max_pt = layer.max_point.max(result.max_point);
                let next = match result.is_empty() {
                    true  => None,
                    false => Some(Arc::new(result)),
                };
                result = Self::create(
                    layer.chunk_pos.clone(),
                    layer.chunk.clone(),
                    next,
                    max_pt,
                );
            }
        }
        result
    }

    /// Returns a new [`RangeSet`] with the additions and/or removals described by `update_spec`,
    /// reusing untouched chunks from `self`.
    pub fn update(&self, update_spec: RangeSetUpdate<'_, T>) -> Self {
        let mut add = update_spec.add;
        let sort = update_spec.sort;
        let filter = update_spec.filter.as_deref();
        let filter_from = update_spec.filter_from.unwrap_or(0);
        let filter_to = update_spec.filter_to.unwrap_or_else(|| self.length());

        if add.is_empty() && filter.is_none() {
            return self.clone();
        }
        if sort {
            add.sort_by(cmp_range);
        }
        if self.is_empty() {
            return match add.is_empty() {
                true  => self.clone(),
                false => Self::of(add, false),
            };
        }

        self.update_inner(&add, filter, filter_from, filter_to)
    }

    fn update_inner(
        &self,
        add: &[Range<T>],
        filter: Option<&dyn Fn(usize, usize, &T) -> bool>,
        filter_from: usize,
        filter_to: usize,
    ) -> Self {
        let mut cur = LayerCursor::new(self, None, -1, 0);
        cur.goto(0, -FAR as i32);
        let mut i = 0;
        let mut spill: Vec<Range<T>> = Vec::new();
        let mut builder = RangeSetBuilder::new();

        while cur.value.is_some() || i < add.len() {
            if i < add.len()
                && cmp_pos_side(
                    cur.from,
                    cur.start_side(),
                    add[i].from as isize,
                    add[i].value.start_side(),
                ) >= 0
            {
                let range = &add[i];
                i += 1;
                if !builder
                    .add_inner(range.from, range.to, range.value.clone())
                    .unwrap()
                {
                    spill.push(range.clone());
                }
            } else if cur.range_index == 1
                && cur.chunk_index < self.chunk.len()
                && (i == add.len() || self.chunk_end(cur.chunk_index) < add[i].from)
                && (filter.is_none()
                    || filter_from > self.chunk_end(cur.chunk_index)
                    || filter_to < self.chunk_pos[cur.chunk_index])
                && builder.add_chunk(
                    self.chunk_pos[cur.chunk_index],
                    Arc::clone(&self.chunk[cur.chunk_index]),
                )
            {
                cur.next_chunk();
            } else {
                let c_from = cur.from as usize;
                let c_to = cur.to as usize;
                let c_val = cur.value.clone().unwrap();
                if filter.is_none()
                    || filter_from > c_to
                    || filter_to < c_from
                    || filter.unwrap()(c_from, c_to, &c_val)
                {
                    if !builder.add_inner(c_from, c_to, c_val.clone()).unwrap() {
                        spill.push(Range::new(c_from, c_to, c_val));
                    }
                }
                cur.advance();
            }
        }

        let next = match &self.next_layer {
            None if spill.is_empty() => Self::empty(),
            None => Self::of(spill, false),
            Some(next_layer) => next_layer.update_inner(&spill, filter, filter_from, filter_to),
        };

        builder.finish_inner(next)
    }

    /// Maps all ranges in this set through `changes`, sharing untouched [`Chunk`]s and dropping
    /// ranges whose [`RangeValue::map_mode`] invalidates them.
    pub fn map(&self, changes: &ChangeDesc) -> Self {
        if changes.is_empty() || self.is_empty() {
            return self.clone();
        }

        let mut chunks: Vec<Arc<Chunk<T>>> = Vec::new();
        let mut chunk_pos: Vec<usize> = Vec::new();
        let mut max_point: isize = -1;

        for i in 0..self.chunk.len() {
            let start = self.chunk_pos[i];
            let chunk = &self.chunk[i];
            let touch = changes.touches_range(start, Some(start + chunk.length()));
            match touch {
                Touched::No => {
                    max_point = max_point.max(chunk.max_point);
                    chunks.push(Arc::clone(chunk));
                    let new_start = changes
                        .map_pos(start, Assoc::Before, MapMode::Simple)
                        .ok()
                        .flatten()
                        .unwrap_or(start);
                    chunk_pos.push(new_start);
                }
                Touched::Yes => {
                    if let Some((pos, mapped)) = chunk.map(start, changes) {
                        max_point = max_point.max(mapped.max_point);
                        chunks.push(mapped);
                        chunk_pos.push(pos);
                    }
                }
                Touched::Cover => {}
            }
        }

        let next = self
            .next_layer
            .as_ref()
            .map(|n| n.map(changes))
            .unwrap_or_else(Self::empty);

        if chunks.is_empty() {
            next
        } else {
            Self::create(chunk_pos, chunks, Some(Arc::new(next)), max_point)
        }
    }

    /// Calls `f(range_from, range_to, &value)` for every range in this set that overlaps `[from..to]`.
    ///
    /// If `f` returns `false`, iteration stops early.
    pub fn between<F>(&self, from: usize, to: usize, mut f: F)
    where
        F: FnMut(usize, usize, &T) -> bool,
    {
        self.between_inner(from, to, &mut f);
    }

    fn between_inner<F>(&self, from: usize, to: usize, f: &mut F)
    where
        F: FnMut(usize, usize, &T) -> bool,
    {
        if self.is_empty() { return; }
        for i in 0..self.chunk.len() {
            let start = self.chunk_pos[i];
            let chunk = &self.chunk[i];
            if to >= start
                && from <= start + chunk.length()
                && !chunk.between(
                    start,
                    (from as isize) - (start as isize),
                    (to as isize) - (start as isize),
                    f,
                )
            {
                return;
            }
        }
        if let Some(next) = &self.next_layer {
            next.between_inner(from, to, f);
        }
    }

    /// Returns a [`RangeSetCursor`] iterating over all ranges in this set from offset `0`.
    pub fn iter(&self) -> RangeSetCursor<'_, T> {
        self.iter_from(0)
    }

    /// Returns a [`RangeSetCursor`] positioned at the first range ending at or after `from`.
    pub fn iter_from(&self, from: usize) -> RangeSetCursor<'_, T> {
        let sets = [self];
        let mut cur = RangeSetCursor::from_sets(&sets, None, -1);
        cur.goto(from);
        cur
    }

    /// Returns a merged [`RangeSetCursor`] iterating over multiple [`RangeSet`]s simultaneously from `from`.
    pub fn iter_sets<'a>(sets: &[&'a RangeSet<T>], from: usize) -> RangeSetCursor<'a, T> {
        let mut cur = RangeSetCursor::from_sets(sets, None, -1);
        cur.goto(from);
        cur
    }
}
