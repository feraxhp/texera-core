//! Persistent rope-backed text buffers and line-based indexing.
//!
//! This module provides the text storage representation ported from `@codemirror/state`:
//!
//! - [`Text`]: An immutable, persistent UTF-8 text rope backed by `ropey::Rope`, supporting
//!   $O(\log N)$ character-offset slicing, line lookup, and edits with structural sharing.
//! - [`Line`]: 1-based line metadata including start character offset (`from`), end offset (`to`),
//!   line length, and string slice access.

use ropey::RopeBuilder;
use ropey::Rope;

pub mod operations;
pub mod display;
pub mod length;
pub mod deref;
pub mod lines;
#[cfg(test)]
mod tests;

pub use self::lines::Line;

/// An immutable, persistent rope-backed document representation.
///
/// `Text` stores UTF-8 characters in a balanced B-tree (`ropey::Rope`), enabling $O(\log N)$
/// slicing, insertion, deletion, and line lookup by character index or line number, while making
/// clones $O(1)$ via structural sharing.
///
/// All positions in `Text` are **character offsets** (`char` indices, 0-based), not byte offsets.
///
/// # Examples
///
/// ## Creating a document from strings or line arrays
///
/// ```rust
/// use texera_core::Text;
///
/// let doc1 = Text::from("hello\nworld");
/// let doc2 = Text::from(["hello", "world"]);
/// assert_eq!(doc1, doc2);
/// assert_eq!(doc1.len_chars(), 11);
/// assert_eq!(doc1.len_lines(), 2);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Text { rope: Rope }

impl Text {
    /// Creates a new, empty [`Text`] document.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::Text;
    ///
    /// let doc = Text::new();
    /// assert_eq!(doc.len_chars(), 0);
    /// assert_eq!(doc.to_string(), "");
    /// ```
    pub fn new() -> Self { Self::default() }

    #[inline]
    pub(crate) fn from_rope(rope: Rope) -> Self { Self { rope } }

    #[inline]
    pub(crate) fn as_rope(&self) -> &Rope { &self.rope }

    #[inline]
    pub(crate) fn into_rope(self) -> Rope { self.rope }
}

impl Default for Text {
    fn default() -> Self {
        Self { rope: Default::default() }
    }
}

impl From<&str> for Text {
    fn from(value: &str) -> Self {
        Self { rope: Rope::from_str(value) }
    }
}

impl From<&String> for Text {
    fn from(value: &String) -> Self {
        Self { rope: Rope::from_str(value) }
    }
}

impl<A> From<&[A]> for Text 
where 
    A: AsRef<str>
{
    fn from(lines: &[A]) -> Self {
        let mut builder = RopeBuilder::new();
        
        for (i, line) in lines.iter().enumerate() {
            builder.append(line.as_ref());
            
            // Append newline for all lines except the last one
            if i < lines.len() - 1 {
                builder.append("\n");
            }
        }
        
        Self { rope: builder.finish() }
    }
}

impl<A, const N: usize> From<&[A; N]> for Text
where
    A: AsRef<str>,
{
    fn from(value: &[A; N]) -> Self { Self::from(value.as_slice()) }
}

impl<A, const N: usize> From<[A; N]> for Text
where
    A: AsRef<str>,
{
    fn from(value: [A; N]) -> Self { Self::from(value.as_slice()) }
}
