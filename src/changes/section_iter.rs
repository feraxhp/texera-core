//! Cursor iterator over change sections for composition and mapping.

use crate::changes::section::Section;

/// An internal streaming cursor iterating over a slice of [`Section`]s.
///
/// Tracks both the current section index and a sub-character offset into that section,
/// allowing incremental consumption along either the original or intermediate/replaced timeline.
pub(crate) struct SectionIter<'a> {
    index: usize,
    offset: usize,
    sections: &'a [Section],
}

impl<'a> SectionIter<'a> {
    /// Constructs a new cursor positioned at the beginning of `sections`.
    pub fn new(sections: &'a [Section]) -> Self {
        Self { sections, index: 0, offset: 0 }
    }

    /// Returns the character offset within the current section.
    #[inline]
    pub fn offset(&self) -> usize { self.offset }

    /// Returns the index of the current section in the underlying slice.
    #[inline]
    pub fn index(&self) -> usize { self.index }

    /// Returns `true` if the iterator has consumed all sections.
    #[inline]
    pub fn is_done(&self) -> bool {
        self.index >= self.sections.len()
    }

    /// Returns a reference to the current section.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if called when [`is_done`](Self::is_done) is `true`.
    #[inline]
    pub fn current(&self) -> &'a Section {
        debug_assert!(self.index < self.sections.len());
        
        unsafe {
            self.sections.get_unchecked(self.index)
        }
    }

    /// Returns `true` if the cursor is currently pointing at a keep section.
    #[inline]
    pub fn is_keep(&self) -> bool {
        !self.is_done() && self.current().is_keep()
    }
    
    /// Returns the number of unconsumed characters in the current section along the original timeline.
    #[inline]
    pub fn remaining_original(&self) -> usize {
        if self.is_done() {
            0
        } else {
            self.current().original - self.offset
        }
    }

    /// Returns the number of unconsumed characters in the current section along the intermediate/replaced timeline.
    #[inline]
    pub fn remaining_on_intermediate(&self) -> usize {
        if self.is_done() {
            0
        } else {
            let s = self.current();
            if s.is_keep() {
                s.original - self.offset
            } else {
                s.replaced - self.offset
            }
        }
    }
    
    /// Advances the cursor by `n` characters along the original document timeline.
    #[inline]
    pub fn forward_original(&mut self, n: usize) {
        if self.is_done() {
            return;
        }
        self.offset += n;
        if self.offset >= self.current().original {
            self.index += 1;
            self.offset = 0;
        }
    }

    /// Advances the cursor by `n` characters along the intermediate/replaced document timeline.
    #[inline]
    pub fn forward_intermediate(&mut self, n: usize) {
        if self.is_done() {
            return;
        }
        self.offset += n;
        let s = self.current();
        let total = if s.is_keep() { s.original } else { s.replaced };
        
        if self.offset >= total {
            self.index += 1;
            self.offset = 0;
        }
    }

    /// Advances the cursor to the start of the next section.
    #[inline]
    pub fn next(&mut self) {
        self.index += 1;
        self.offset = 0;
    }
}
