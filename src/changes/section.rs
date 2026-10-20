//! Compressed change section representations for operational transformations.

/// An atomic section within a change sequence describing how a span of text transitions.
///
/// A section records:
/// - `original`: The character length in the input document before the change.
/// - `replaced`: The character length in the output document, or [`Section::keep`] if unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Section {
    /// Character length covered in the original document.
    pub(crate) original: usize,
    /// Character length in the replacement text, or [`Section::keep`] if unchanged.
    pub(crate) replaced: usize,
}

impl Section {
    /// Sentinel value indicating that the original text is preserved without modifications.
    #[allow(non_upper_case_globals)]
    pub const keep: usize = usize::MAX;
    
    /// Returns `true` if this section leaves the underlying text unchanged.
    #[inline]
    pub fn is_keep(&self) -> bool { self.replaced == Self::keep }

    /// Returns `true` if this section represents a pure text deletion.
    #[inline]
    pub fn is_deletion(&self) -> bool { self.replaced == 0 && self.original > 0 }
    
    /// Returns `true` if this section represents a pure text insertion.
    #[inline]
    pub fn is_insertion(&self) -> bool { self.replaced > 0 && self.original == 0 }
    
    /// Inverts this section, swapping `original` and `replaced` for non-keep sections.
    #[inline]
    pub fn invert(&self) -> Self {
        if self.is_keep() { *self }
        else { 
            Self { 
                original: self.replaced, 
                replaced: self.original, 
            }
        }
    }
}
