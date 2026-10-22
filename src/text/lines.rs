use std::borrow::Cow;
use mynox::Error;
use ropey::{Rope, RopeSlice};

use crate::errors::out_of_bounds;
use super::super::Text;

/// Metadata and content for a single line in a [`Text`] document.
///
/// Returned by [`Text::line`], [`Text::line_at`], [`Text::iter_lines`], and [`Text::iter_range`].
///
/// Backed directly by a zero-allocation [`RopeSlice`], avoiding heap allocations when querying
/// line metadata or slicing text.
///
/// # Examples
///
/// ```rust
/// use texera_core::Text;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let doc = Text::from("first\nsecond");
/// let line0 = doc.line(0)?;
/// assert_eq!(line0.number(), 0);
/// assert_eq!(line0.from(), 0);
/// assert_eq!(line0.text_trimmed(), "first");
/// assert_eq!(line0.length_trimmed(), 5);
/// assert_eq!(line0.to_trimmed(), 5);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Line<'a> {
    from: usize,
    to: usize,
    number: usize,
    slice: RopeSlice<'a>,
}

impl<'a> Line<'a> {    
    /// Returns the start character offset of this line in the document.
    #[inline]
    pub fn from(&self) -> usize { self.from }

    /// Returns the end character offset of this line (including any trailing newline character).
    #[inline]
    pub fn to(&self) -> usize { self.to }

    /// Returns the 0-based line index in the document.
    #[inline]
    pub fn number(&self) -> usize { self.number }

    /// Returns the character length of the line (including any trailing newline character).
    #[inline]
    pub fn length(&self) -> usize { self.to - self.from }

    /// Returns the underlying [`RopeSlice`] representing this line.
    #[inline]
    pub fn slice(&self) -> RopeSlice<'a> { self.slice }

    /// Returns the string slice if this line is stored in a contiguous chunk in the rope.
    #[inline]
    pub fn as_str(&self) -> Option<&'a str> { self.slice.as_str() }

    /// Returns the full text of the line (including any trailing newline character).
    ///
    /// Borrows without allocation if the line is contiguous in a chunk; allocates a [`String`]
    /// only if the line spans multiple chunks.
    pub fn text(&self) -> Cow<'a, str> {
        match self.slice.as_str() {
            Some(s) => Cow::Borrowed(s),
            None => Cow::Owned(self.slice.to_string()),
        }
    }

    /// Number of trailing line-break characters (`\r\n` = 2, `\n` = 1, `\r` = 1, none = 0).
    fn newline_chars(&self) -> usize {
        if let Some(s) = self.slice.as_str() {
            let bytes = s.as_bytes();
            let len = bytes.len();
            if len == 0 {
                return 0;
            }
            if bytes[len - 1] == b'\n' {
                if len >= 2 && bytes[len - 2] == b'\r' {
                    2
                } else {
                    1
                }
            } else if bytes[len - 1] == b'\r' {
                1
            } else {
                0
            }
        } else {
            let len = self.slice.len_chars();
            if len == 0 {
                return 0;
            }
            if let Some(last_chunk) = self.slice.chunks().last() {
                let bytes = last_chunk.as_bytes();
                let chunk_len = bytes.len();
                if chunk_len > 0 && bytes[chunk_len - 1] == b'\n' {
                    if chunk_len >= 2 && bytes[chunk_len - 2] == b'\r' {
                        return 2;
                    } else if chunk_len == 1 && len >= 2 && self.slice.char(len - 2) == '\r' {
                        return 2;
                    } else {
                        return 1;
                    }
                } else if chunk_len > 0 && bytes[chunk_len - 1] == b'\r' {
                    return 1;
                } else {
                    return 0;
                }
            }
            0
        }
    }

    /// Returns the character length of the line excluding trailing line breaks.
    #[inline]
    pub fn length_trimmed(&self) -> usize {
        self.length().saturating_sub(self.newline_chars())
    }

    /// Returns the end character offset of the line excluding trailing line breaks.
    #[inline]
    pub fn to_trimmed(&self) -> usize {
        self.from + self.length_trimmed()
    }

    /// Returns the sub-slice of this line without trailing line breaks.
    #[inline]
    pub fn slice_trimmed(&self) -> RopeSlice<'a> {
        self.slice.slice(..self.length_trimmed())
    }

    /// Returns the line text without trailing line break characters (`\r\n`, `\n`, or `\r`).
    ///
    /// Borrows without allocation if the line is contiguous in a chunk; allocates a [`String`]
    /// only if the line spans multiple chunks.
    pub fn text_trimmed(&self) -> Cow<'a, str> {
        let trimmed = self.slice_trimmed();
        match trimmed.as_str() {
            Some(s) => Cow::Borrowed(s),
            None => Cow::Owned(trimmed.to_string()),
        }
    }
}

impl<'a> std::fmt::Display for Line<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.slice)
    }
}

impl<'a> From<(&'a Rope, usize)> for Line<'a> {
    fn from((rope, line_number): (&'a Rope, usize)) -> Self {
        let slice = rope.line(line_number);
        let from = rope.line_to_char(line_number);
        let to = from + slice.len_chars();

        Line { 
            from, 
            to, 
            number: line_number, 
            slice,
        }
    }
}

impl Text {
    /// Returns the [`Line`] descriptor for the given 0-based line `number`.
    ///
    /// Returns an `OutOfBounds` error if `number >= self.len_lines()`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::Text;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let doc = Text::from(["alpha", "beta"]);
    /// let line1 = doc.line(1)?;
    /// assert_eq!(line1.from(), 6);
    /// assert_eq!(line1.text_trimmed(), "beta");
    /// assert!(doc.line(2).is_err());
    /// # Ok(())
    /// # }
    /// ```
    pub fn line(&self, number: usize) -> Result<Line<'_>, Error> {
        let lines = self.len_lines();
        if number >= lines { return Err(out_of_bounds(number, lines.saturating_sub(1))); }
        Ok((&self.rope, number).into())
    }

    /// Returns the [`Line`] containing the given character offset `pos`.
    ///
    /// Returns an `OutOfBounds` error if `pos > self.len_chars()`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::Text;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let doc = Text::from("abc\ndef");
    /// let line = doc.line_at(5)?;
    /// assert_eq!(line.number(), 1);
    /// assert_eq!(line.text_trimmed(), "def");
    /// # Ok(())
    /// # }
    /// ```
    pub fn line_at(&self, pos: usize) -> Result<Line<'_>, Error> {
        let chars = self.len_chars();
        if chars < pos { return Err(out_of_bounds(pos, chars)); }
        let number = self.rope.char_to_line(pos);
        
        Ok((&self.rope, number).into())
    }

    /// Iterates over [`Line`]s in the inclusive line index range `[from..=to]`.
    ///
    /// - `from`: Starting 0-based line index (defaults to `0`).
    /// - `to`: Ending 0-based line index, inclusive (defaults to the last line).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::Text;
    ///
    /// let doc = Text::from(["l0", "l1", "l2", "l3"]);
    /// let subset: Vec<String> = doc
    ///     .iter_lines(Some(1), Some(2))
    ///     .map(|l| l.text_trimmed().to_string())
    ///     .collect();
    /// assert_eq!(subset, vec!["l1", "l2"]);
    /// ```
    pub fn iter_lines(&self, from: Option<usize>, to: Option<usize>) -> impl Iterator<Item = Line<'_>> + '_ {
        let from_line = match from {
            Some(pos) => pos,
            None => 0,
        };
        
        let to_line = match to {
            Some(pos) => pos + 1, 
            None => self.rope.len_lines(),
        };
        
        let end_line = to_line.min(self.rope.len_lines()).max(from_line);
        let total_lines = end_line.saturating_sub(from_line);

        let mut char_offset = if total_lines > 0 {
            self.rope.line_to_char(from_line)
        } else {
            0
        };

        let lines_iter = self.rope.lines_at(from_line);

        lines_iter.take(total_lines).enumerate().map(move |(idx, slice)| {
            let line_num = from_line + idx;
            let from = char_offset;
            let len = slice.len_chars();
            char_offset += len;
            let to = from + len;
            Line {
                from,
                to,
                number: line_num,
                slice,
            }
        })
    }
    
    /// Iterates over all [`Line`]s touched by the character range `[from..=to]`.
    ///
    /// - `from`: Starting character offset (defaults to `0`).
    /// - `to`: Ending character offset (defaults to the end of the document).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::Text;
    ///
    /// let doc = Text::from("aa\nbb\ncc");
    /// let touched: Vec<usize> = doc.iter_range(Some(1), Some(4)).map(|l| l.number()).collect();
    /// assert_eq!(touched, vec![0, 1]);
    /// ```
    pub fn iter_range(&self, from: Option<usize>, to: Option<usize>) -> impl Iterator<Item = Line<'_>> + '_ {
        let len_chars = self.rope.len_chars();
        let from_line = match from {
            Some(pos) => self.rope.char_to_line(pos.min(len_chars)),
            None => 0,
        };
        
        let to_line = match to {
            Some(pos) => self.rope.char_to_line(pos.min(len_chars)) + 1, 
            None => self.rope.len_lines(),
        };
        
        let end_line = to_line.min(self.rope.len_lines()).max(from_line);
        let total_lines = end_line.saturating_sub(from_line);

        let mut char_offset = if total_lines > 0 {
            self.rope.line_to_char(from_line)
        } else {
            0
        };

        let lines_iter = self.rope.lines_at(from_line);

        lines_iter.take(total_lines).enumerate().map(move |(idx, slice)| {
            let line_num = from_line + idx;
            let from = char_offset;
            let len = slice.len_chars();
            char_offset += len;
            let to = from + len;
            Line {
                from,
                to,
                number: line_num,
                slice,
            }
        })
    }
}
