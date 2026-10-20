use crate::MapMode;

use super::Range;


/// Trait implemented by values stored inside a [`RangeSet`](crate::RangeSet).
///
/// Controls boundary inclusivity (`start_side` / `end_side`), deletion mapping behavior
/// (`map_mode`), and whether the range acts as an atomic point decoration (`point`).
///
/// # Examples
///
/// ```rust
/// use texera_core::{MapMode, RangeValue};
///
/// #[derive(Clone, Debug, PartialEq, Eq)]
/// struct Widget {
///     id: u32,
///     inclusive: bool,
/// }
///
/// impl RangeValue for Widget {
///     fn eq(&self, other: &Self) -> bool { self == other }
///     fn start_side(&self) -> i32 { if self.inclusive { -1 } else { 1 } }
///     fn end_side(&self) -> i32 { if self.inclusive { 1 } else { -1 } }
///     fn map_mode(&self) -> MapMode { MapMode::TrackDel }
///     fn point(&self) -> bool { true }
/// }
///
/// let r = Widget { id: 1, inclusive: false }.point_range(5);
/// assert_eq!(r.from, 5);
/// assert_eq!(r.to, 5);
/// ```
pub trait RangeValue: Clone + Send + Sync + 'static {
    /// Compares two range values for logical equality (used by [`RangeSet::compare`](crate::RangeSet::compare)
    /// and [`RangeSet::eq_sets`](crate::RangeSet::eq_sets)).
    fn eq(&self, other: &Self) -> bool;

    /// Relative side of the start boundary. Negative values mean the range includes text inserted
    /// at `from` (`Assoc::Before`), positive values exclude it (`Assoc::After`). Defaults to `0`.
    fn start_side(&self) -> i32 { 0 }

    /// Relative side of the end boundary. Positive values mean the range includes text inserted
    /// at `to` (`Assoc::After`), negative values exclude it (`Assoc::Before`). Defaults to `0`.
    fn end_side(&self) -> i32 { 0 }

    /// Determines how the range behaves when the text it covers is deleted. Defaults to [`MapMode::TrackDel`].
    fn map_mode(&self) -> MapMode { MapMode::TrackDel }

    /// Returns `true` if this value represents a point decoration or atomic replacement widget.
    fn point(&self) -> bool { false }

    /// Wraps `self` into a [`Range`] spanning `[from..to]`.
    fn range(self, from: usize, to: usize) -> Range<Self> {
        Range::new(from, to, self)
    }

    /// Wraps `self` into a zero-length point [`Range`] at `[pos..pos]`.
    fn point_range(self, pos: usize) -> Range<Self> {
        Range::new(pos, pos, self)
    }
}

/// Visitor callback trait used by [`RangeSet::compare`](crate::RangeSet::compare) to receive
/// notifications of intervals where active ranges or point decorations differ between two sets of [`RangeSet`](crate::RangeSet)s.
pub trait RangeComparator<T: RangeValue> {
    /// Called when the set of active non-point ranges over `[from..to]` differs between the old and new sets.
    fn compare_range(&mut self, from: usize, to: usize, active_a: &[T], active_b: &[T]);

    /// Called when a point decoration over `[from..to]` was added, removed, or changed.
    fn compare_point(
        &mut self,
        from: usize,
        to: usize,
        point_a: Option<&T>,
        point_b: Option<&T>,
    );

    /// Called when a range boundary changes at `pos` (only invoked if [`RangeComparator::tracks_bound_change`] returns `true`).
    fn bound_change(&mut self, _pos: usize) {}
    /// Return `true` to receive [`RangeComparator::bound_change`] notifications.
    fn tracks_bound_change(&self) -> bool { false }
}

/// Visitor callback trait used by [`RangeSet::spans`](crate::RangeSet::spans) to partition a document
/// interval into uniform spans of active ranges and atomic point ranges.
///
/// # Examples
///
/// ```rust
/// use texera_core::{RangeSet, RangeValue, SpanIterator};
///
/// #[derive(Clone, Debug, PartialEq, Eq)]
/// struct Style(&'static str);
/// impl RangeValue for Style {
///     fn eq(&self, other: &Self) -> bool { self == other }
/// }
///
/// struct Collector(Vec<(usize, usize, Vec<&'static str>)>);
/// impl SpanIterator<Style> for Collector {
///     fn span(&mut self, from: usize, to: usize, active: &[Style], _open_start: usize) {
///         self.0.push((from, to, active.iter().map(|s| s.0).collect()));
///     }
///     fn point(&mut self, _from: usize, _to: usize, _value: &Style, _active: &[Style], _open: usize, _idx: usize) {}
/// }
///
/// let set = RangeSet::of([Style("bold").range(2, 6)], false);
/// let mut col = Collector(Vec::new());
/// RangeSet::spans(&[&set], 0, 8, &mut col, None);
/// assert_eq!(
///     col.0,
///     vec![(0, 2, vec![]), (2, 6, vec!["bold"]), (6, 8, vec![])]
/// );
/// ```
pub trait SpanIterator<T: RangeValue> {
    /// Called for each contiguous sub-interval `[from..to]` not interrupted by a point decoration,
    /// providing the slice of currently `active` range values.
    fn span(&mut self, from: usize, to: usize, active: &[T], open_start: usize);

    /// Called when a point decoration (`point() == true`) is encountered covering `[from..to]`.
    fn point(
        &mut self,
        from: usize,
        to: usize,
        value: &T,
        active: &[T],
        open_start: usize,
        index: usize,
    );
}
