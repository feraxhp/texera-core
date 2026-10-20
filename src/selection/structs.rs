/// Side association for cursor positions and position mapping.
///
/// Determines whether a cursor or position at a character boundary is associated with the
/// character [`Assoc::Before`] or [`Assoc::After`] it (relevant for wrapped lines, bidirectional
/// text, and mapping positions across insertions).
///
/// # Examples
///
/// ```rust
/// use texera_core::Assoc;
///
/// assert!(Assoc::Before.is_before());
/// assert!(Assoc::After.is_after());
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Assoc {
    /// Associated with the character before the position (sticks to the left on insertion).
    Before,
    /// Associated with the character after the position (moves to the right on insertion).
    After,
}

/// A single selection range or cursor within an editor [`Selection`].
///
/// A `SelectionRange` is defined by an `anchor` (the fixed end where the selection started)
/// and a `head` (the moving end where the caret is located).
///
/// - `from()` is always `anchor.min(head)`.
/// - `to()` is always `anchor.max(head)`.
/// - When `anchor == head`, the range is an empty cursor (`empty() == true`).
/// - When `head < anchor`, the selection is directed backwards.
///
/// # Examples
///
/// ```rust
/// use texera_core::{Selection, SelectionRange};
///
/// let cursor = SelectionRange::cursor(5);
/// assert!(cursor.empty());
/// assert_eq!(cursor.from(), 5);
///
/// let backward = Selection::range(10, 4);
/// assert!(!backward.empty());
/// assert_eq!(backward.from(), 4);
/// assert_eq!(backward.to(), 10);
/// assert_eq!(backward.anchor(), 10);
/// assert_eq!(backward.head(), 4);
/// ```
#[derive(Clone, Copy, Debug, Eq)]
pub struct SelectionRange {
    pub(crate) from: usize,
    pub(crate) to: usize,
    pub(crate) column: Option<usize>,
    pub(crate) assoc: Assoc,
    pub(crate) anchor: usize,
    pub(crate) head: usize,
}

/// An immutable editor selection consisting of one or more non-overlapping, sorted
/// [`SelectionRange`]s and a designated primary (`main`) range index.
///
/// Whenever a `Selection` is constructed via [`Selection::from`] or [`Selection::from_with_head`],
/// overlapping or adjacent ranges are automatically sorted and merged so the selection invariant
/// always holds.
///
/// # Examples
///
/// ```rust
/// use texera_core::Selection;
///
/// let sel = Selection::from([
///     Selection::range(5, 8),
///     Selection::range(0, 2),
/// ]);
/// assert_eq!(sel.count(), 2);
/// assert_eq!(sel.ranges_ref()[0].from(), 0);
/// assert_eq!(sel.main().from(), 5);
/// ```
#[derive(Clone, Debug)]
pub struct Selection {
    pub(crate) ranges: Vec<SelectionRange>,
    pub(crate) main: usize,
}

/// Optional selection specification wrapper.
#[derive(Clone, Debug)]
pub enum SelectionSpec {
    /// A concrete [`Selection`].
    Selection(Selection),
    /// No selection specified.
    None,
}

impl Assoc {
    /// Returns `true` if `self` is [`Assoc::Before`].
    #[inline]
    pub fn is_before(&self) -> bool { matches!(self, Self::Before) }
    /// Returns `true` if `self` is [`Assoc::After`].
    #[inline]
    pub fn is_after(&self) -> bool { matches!(self, Self::After) }
}
