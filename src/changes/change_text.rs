//! Generic text payload abstraction for change descriptions and sets.

use crate::Text;

/// Abstraction over the text content attached to an edit section.
///
/// Implemented for:
/// - [`Text`]: Used by [`ChangeSet`](crate::ChangeSet) to store the actual inserted document ropes.
/// - `()`: Used by [`ChangeDesc`](crate::ChangeDesc) for lightweight change tracking without text storage.
pub trait ChangeText: Clone + Default {
    /// Appends the content of `other` to `self`.
    fn append(&mut self, other: Self);
    /// Returns `true` if this payload contains no characters.
    fn is_empty(&self) -> bool;
}

impl ChangeText for () {
    #[inline(always)]
    fn append(&mut self, _: ()) {}
    #[inline(always)]
    fn is_empty(&self) -> bool { true }
}

impl ChangeText for Text {
    #[inline(always)]
    fn append(&mut self, other: Self) {
        if other.len_chars() > 0 {
            *self = self.clone().append(other);
        }
    }
    #[inline(always)]
    fn is_empty(&self) -> bool { self.len_chars() == 0 }
}
