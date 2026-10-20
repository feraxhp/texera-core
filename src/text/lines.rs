use mynox::Error;
use ropey::Rope;

use crate::errors::out_of_bounds;
use super::super::Text;

/// Metadata and content for a single line in a [`Text`] document.
///
/// Returned by [`Text::line`], [`Text::line_at`], [`Text::iter_lines`], and [`Text::iter_range`].
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
#[derive(Clone, Debug)]
pub struct Line {
    from: usize,
    to: usize,
    number: usize,
    text: String,
    length: usize,
}

impl Line {    
    /// Returns the start character offset of this line in the document.
    pub fn from(&self) -> usize { self.from }
    /// Returns the end character offset of this line (including any trailing newline character).
    pub fn to(&self) -> usize { self.to }
    /// Returns the 0-based line index in the document.
    pub fn number(&self) -> usize { self.number }
    /// Returns the full string of the line (including any trailing newline character).
    pub fn text(&self) -> String { self.text.to_owned() }
    /// Returns the character length of the line (including any trailing newline character).
    pub fn length(&self) -> usize { self.length }

    /// Returns the line text without trailing line break characters (`\r\n`, `\n`, or `\r`).
    pub fn text_trimmed(&self) -> &str {
        self.text
            .strip_suffix("\r\n")
            .or_else(|| self.text.strip_suffix('\n'))
            .or_else(|| self.text.strip_suffix('\r'))
            .unwrap_or(&self.text)
    }

    /// Returns the character length of the line excluding trailing line breaks.
    pub fn length_trimmed(&self) -> usize {
        let trimmed_bytes = self.text.len() - self.text_trimmed().len();
        let newline_chars = match trimmed_bytes {
            2 => 2,
            1 => 1,
            _ => 0,
        };
        self.length.saturating_sub(newline_chars)
    }

    /// Returns the end character offset of the line excluding trailing line breaks.
    pub fn to_trimmed(&self) -> usize {
        self.from + self.length_trimmed()
    }
}

impl From<(&Rope, usize)> for Line {
    fn from((rope, line_number): (&Rope, usize)) -> Self {
        let slice = rope.line(line_number);
        
        let length = slice.len_chars();
        let from = rope.line_to_char(line_number);
        let to = from + length;

        Line { 
            from, 
            to, 
            number: line_number, 
            text: slice.to_string(), 
            length,
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
    pub fn line(&self, number: usize) -> Result<Line, Error> {
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
    pub fn line_at(&self, pos: usize) -> Result<Line, Error> {
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
    pub fn iter_lines(&self, from: Option<usize>, to: Option<usize>) -> impl Iterator<Item = Line> + '_ {
        let from_line = match from {
            Some(pos) => pos,
            None => 0,
        };
        
        let to_line = match to {
            Some(pos) => pos + 1, 
            None => self.rope.len_lines(),
        };
        
        let end_line = to_line.min(self.rope.len_lines()).max(from_line);
        
        let rope = &self.rope;
        
        (from_line..end_line).map(move |line_idx| (rope, line_idx).into())
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
    pub fn iter_range(&self, from: Option<usize>, to: Option<usize>) -> impl Iterator<Item = Line> + '_ {
        let from_line = match from {
            Some(pos) => self.rope.char_to_line(pos),
            None => 0,
        };
        
        let to_line = match to {
            Some(pos) => self.rope.char_to_line(pos) + 1, 
            None => self.rope.len_lines(),
        };
        
        let end_line = to_line.min(self.rope.len_lines()).max(from_line);
        
        let rope = &self.rope;
        
        (from_line..end_line).map(move |line_idx| (rope, line_idx).into())
    }
}
