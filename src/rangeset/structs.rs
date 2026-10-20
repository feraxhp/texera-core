use std::collections::HashSet;
use std::sync::Arc;

use super::RangeValue;


/// A half-open document interval `[from..to]` associated with a typed [`RangeValue`] payload `T`.
///
/// Point ranges (such as inline widget decorations or cursor markers) have `from == to`.
///
/// # Examples
///
/// ```rust
/// use texera_core::{Range, RangeValue};
///
/// #[derive(Clone, Debug, PartialEq, Eq)]
/// struct Token(&'static str);
/// impl RangeValue for Token {
///     fn eq(&self, other: &Self) -> bool { self == other }
/// }
///
/// let r = Range::new(2, 6, Token("ident"));
/// assert_eq!(r.from, 2);
/// assert_eq!(r.to, 6);
/// assert_eq!(r.value, Token("ident"));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Range<T: RangeValue> {
    /// Start character offset (inclusive).
    pub from: usize,
    /// End character offset (exclusive).
    pub to: usize,
    /// Associated value payload.
    pub value: T,
}

impl<T: RangeValue> Range<T> {
    /// Creates a new [`Range`] covering `[from..to]` with `value`.
    ///
    /// # Panics
    ///
    /// Panics if `from > to`.
    pub fn new(from: usize, to: usize, value: T) -> Self {
        assert!(
            from <= to,
            "Range `from` ({from}) cannot be greater than `to` ({to})"
        );
        Self { from, to, value }
    }
}

/// An internal B-tree leaf chunk storing up to 250 non-overlapping [`Range`]s with relative offsets.
#[derive(Clone, Debug)]
pub struct Chunk<T: RangeValue> {
    pub(crate) from: Vec<usize>,
    pub(crate) to: Vec<usize>,
    pub(crate) value: Vec<T>,
    pub(crate) max_point: isize,
}

/// Specification passed to [`RangeSet::update`] to add new ranges and/or filter out existing ranges.
///
/// # Examples
///
/// ```rust
/// use texera_core::{RangeSet, RangeSetUpdate, RangeValue};
///
/// #[derive(Clone, Debug, PartialEq, Eq)]
/// struct Mark(u8);
/// impl RangeValue for Mark {
///     fn eq(&self, other: &Self) -> bool { self == other }
/// }
///
/// let set = RangeSet::of([Mark(1).range(0, 4), Mark(2).range(6, 10)], false);
/// let updated = set.update(RangeSetUpdate {
///     add: vec![Mark(3).range(2, 5)],
///     sort: true,
///     filter: Some(Box::new(|from, _to, _val| from != 6)),
///     filter_from: Some(5),
///     filter_to: Some(10),
/// });
///
/// let items: Vec<_> = updated.iter().map(|r| (r.from, r.to, r.value.0)).collect();
/// assert_eq!(items, vec![(0, 4, 1), (2, 5, 3)]);
/// ```
pub struct RangeSetUpdate<'a, T: RangeValue> {
    /// New ranges to insert into the set.
    pub add: Vec<Range<T>>,
    /// If `true`, `add` will be sorted by position and `start_side` before insertion.
    pub sort: bool,
    /// Optional predicate returning `false` for existing ranges within `[filter_from..filter_to]` that should be removed.
    pub filter: Option<Box<dyn Fn(usize, usize, &T) -> bool + 'a>>,
    /// Lower bound of the region where `filter` is applied (defaults to `0`).
    pub filter_from: Option<usize>,
    /// Upper bound of the region where `filter` is applied (defaults to `self.length()`).
    pub filter_to: Option<usize>,
}

impl<'a, T: RangeValue> Default for RangeSetUpdate<'a, T> {
    fn default() -> Self {
        Self {
            add: Vec::new(),
            sort: false,
            filter: None,
            filter_from: None,
            filter_to: None,
        }
    }
}

/// A persistent, layered collection of intervals ([`Range<T>`]) supporting fast position mapping,
/// incremental updates, diff comparison, and span iteration.
///
/// Internally, `RangeSet` partitions non-overlapping ranges into [`Arc<Chunk<T>>`](Chunk) leaves
/// and spills overlapping ranges into linked `next_layer` sets. Unmodified chunks are shared
/// across [`RangeSet::map`] and [`RangeSet::update`] calls, making [`RangeSet::compare`] able to
/// skip entire unchanged subtrees in $O(1)$.
///
/// # Examples
///
/// ```rust
/// use texera_core::{ChangeSet, ChangeSpec, RangeSet, RangeValue};
///
/// #[derive(Clone, Debug, PartialEq, Eq)]
/// struct Fold;
/// impl RangeValue for Fold {
///     fn eq(&self, other: &Self) -> bool { self == other }
/// }
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let set = RangeSet::of([Fold.range(2, 8), Fold.range(10, 15)], false);
/// assert_eq!(set.size(), 2);
///
/// // Insert 3 characters at the start of the document
/// let cs = ChangeSet::from_slice(&[ChangeSpec::new(0, None, Some("abc".into()))?], 20)?;
/// let mapped = set.map(cs.description());
///
/// let coords: Vec<_> = mapped.iter().map(|r| (r.from, r.to)).collect();
/// assert_eq!(coords, vec![(5, 11), (13, 18)]);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct RangeSet<T: RangeValue> {
    pub(crate) chunk_pos: Vec<usize>,
    pub(crate) chunk: Vec<Arc<Chunk<T>>>,
    pub(crate) next_layer: Option<Arc<RangeSet<T>>>,
    pub(crate) max_point: isize,
}

/// Incremental builder for constructing a [`RangeSet<T>`] from ranges added in sorted order.
///
/// Overlapping ranges are automatically routed into secondary layers.
///
/// # Examples
///
/// ```rust
/// use texera_core::{RangeSetBuilder, RangeValue};
///
/// #[derive(Clone, Debug, PartialEq, Eq)]
/// struct Deco(&'static str);
/// impl RangeValue for Deco {
///     fn eq(&self, other: &Self) -> bool { self == other }
/// }
///
/// let mut builder = RangeSetBuilder::new();
/// builder.add(0, 5, Deco("a"));
/// builder.add(2, 4, Deco("b")); // overlaps [0..5], placed in next layer automatically
/// let set = builder.finish();
/// assert_eq!(set.size(), 2);
/// ```
pub struct RangeSetBuilder<T: RangeValue> {
    pub(crate) chunks: Vec<Arc<Chunk<T>>>,
    pub(crate) chunk_pos: Vec<usize>,
    pub(crate) chunk_start: isize,
    pub(crate) last: Option<T>,
    pub(crate) last_from: isize,
    pub(crate) last_to: isize,
    pub(crate) from: Vec<usize>,
    pub(crate) to: Vec<usize>,
    pub(crate) value: Vec<T>,
    pub(crate) max_point: isize,
    pub(crate) set_max_point: isize,
    pub(crate) next_layer: Option<Box<RangeSetBuilder<T>>>,
}

#[derive(Clone)]
pub(crate) struct LayerCursor<'a, T: RangeValue> {
    pub(crate) layer: &'a RangeSet<T>,
    pub(crate) skip: Option<&'a HashSet<usize>>,
    pub(crate) min_point: isize,
    pub(crate) rank: usize,
    pub(crate) from: isize,
    pub(crate) to: isize,
    pub(crate) value: Option<T>,
    pub(crate) chunk_index: usize,
    pub(crate) range_index: usize,
}

/// A min-heap cursor that merges and iterates over ranges across one or more [`RangeSet`]s
/// (and their internal layers) in sorted order.
///
/// Implements [`Iterator<Item = Range<T>>`](Iterator) and also supports manual cursor navigation
/// via [`RangeSetCursor::goto`] and [`RangeSetCursor::advance`].
pub struct RangeSetCursor<'a, T: RangeValue> {
    pub(crate) heap: Vec<LayerCursor<'a, T>>,
    pub(crate) from: isize,
    pub(crate) to: isize,
    /// Value of the current range under the cursor, or `None` when exhausted.
    pub value: Option<T>,
    /// Index of the origin [`RangeSet`] in the slice passed to [`RangeSet::iter_sets`].
    pub rank: usize,
}

pub(crate) struct SpanCursor<'a, T: RangeValue> {
    pub(crate) cursor: RangeSetCursor<'a, T>,
    pub(crate) active: Vec<T>,
    pub(crate) active_to: Vec<isize>,
    pub(crate) active_rank: Vec<usize>,
    pub(crate) min_active: isize,
    pub(crate) point: Option<T>,
    pub(crate) point_from: usize,
    pub(crate) point_rank: usize,
    pub(crate) to: isize,
    pub(crate) end_side: i32,
    pub(crate) open_start: isize,
}
