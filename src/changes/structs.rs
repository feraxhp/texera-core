use crate::Text;


/// Mapping of a single character offset from an `old` document to a `new` document.
pub type Position = ChangeMap<usize>;

/// Pairs an `old` value (before changes) with a `new` value (after changes).
///
/// # Examples
///
/// ```rust
/// use texera_core::ChangeMap;
///
/// let mapped = ChangeMap { old: 2usize, new: 5usize };
/// assert_eq!(mapped.old, 2);
/// assert_eq!(mapped.new, 5);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChangeMap<T> {
    /// Coordinate or range in the original document.
    pub old: T,
    /// Coordinate or range in the modified document.
    pub new: T,
}

/// A half-open character offset range `[from..to]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Range {
    /// Start character offset (inclusive).
    pub from: usize,
    /// End character offset (exclusive).
    pub to: usize,
}

/// Describes an unchanged span of `length` characters mapped between old and new coordinates.
///
/// Yielded by [`ChangeIterator::iter_unchanged`](crate::ChangeIterator::iter_unchanged).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SimpleChange<T> {
    /// Number of unchanged characters in this span.
    pub length: usize,
    /// Start position mapping (`old` -> `new`) for this unchanged span.
    pub map: T,
}

/// Indicates whether and how a [`ChangeDesc`](crate::ChangeDesc) intersects a given range.
///
/// Returned by [`ChangeDesc::touches_range`](crate::ChangeDesc::touches_range).
///
/// # Examples
///
/// ```rust
/// use texera_core::{ChangeSet, ChangeSpec, Touched};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let cs = ChangeSet::from_slice(&[ChangeSpec::new(2, Some(8), None)?], 10)?;
/// let desc = cs.description();
///
/// assert_eq!(desc.touches_range(0, Some(1)), Touched::No);
/// assert_eq!(desc.touches_range(1, Some(4)), Touched::Yes);
/// assert_eq!(desc.touches_range(3, Some(6)), Touched::Cover);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Touched {
    /// No changes overlap with the queried range.
    No,
    /// At least one change overlaps with the queried range, without a single change completely
    /// enclosing it.
    Yes,
    /// A single replacement/deletion completely covers and encloses the queried range.
    Cover,
}


/// A concrete replacement yielded by [`ChangeSet::iter_changes`](crate::ChangeIterator::iter_changes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change<T> {
    /// Old and new range bounds of the replacement.
    pub map: ChangeMap<T>,
    /// Inserted replacement text.
    pub text: Text,
}
