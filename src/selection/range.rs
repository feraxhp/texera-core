use mynox::Error;

use crate::changes::{ChangeDesc, MapMode};

use super::Assoc;
use super::Selection;
use super::SelectionRange;


impl SelectionRange {
    /// Creates an empty cursor [`SelectionRange`] at `pos` (`anchor == head == pos`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::SelectionRange;
    ///
    /// let cur = SelectionRange::cursor(3);
    /// assert!(cur.empty());
    /// assert_eq!(cur.head(), 3);
    /// ```
    pub fn cursor(pos: usize) -> Self {
        SelectionRange { 
            from: pos, 
            to: pos, 
            column: Some(pos), 
            assoc: Assoc::Before,
            anchor: pos, 
            head: pos,
        }
    }

    /// Creates an empty cursor [`SelectionRange`] at `pos` with an explicit [`Assoc`] side.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::{Assoc, SelectionRange};
    ///
    /// let cur = SelectionRange::cursor_with_assoc(3, Assoc::After);
    /// assert_eq!(cur.assoc(), Assoc::After);
    /// ```
    pub fn cursor_with_assoc(pos: usize, assoc: Assoc) -> Self {
        SelectionRange {
            from: pos,
            to: pos,
            column: Some(pos),
            assoc,
            anchor: pos,
            head: pos,
        }
    }
    
    /// Creates a [`SelectionRange`] from `anchor` to `head`.
    ///
    /// `anchor` may be greater than `head` to represent a backward selection.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::SelectionRange;
    ///
    /// let r = SelectionRange::range(8, 2);
    /// assert_eq!(r.from(), 2);
    /// assert_eq!(r.to(), 8);
    /// assert_eq!(r.anchor(), 8);
    /// assert_eq!(r.head(), 2);
    /// ```
    pub fn range(anchor: usize, head: usize) -> Self {
        let (from, to) = match head > anchor {
            false => (head, anchor),
            true => (anchor, head),
        };
        
        SelectionRange { 
            from, to, 
            column: Some(head), 
            assoc: Assoc::Before,
            anchor, head,
        }
    }

    /// Returns a copy of this range with its [`Assoc`] side updated.
    pub fn with_assoc(mut self, assoc: Assoc) -> Self {
        self.assoc = assoc;
        self
    }

    /// Returns a copy of this range with its remembered goal visual `column` updated.
    pub fn with_column(mut self, column: Option<usize>) -> Self {
        self.column = column;
        self
    }
    
    /// Returns the lower bound of the range (`anchor.min(head)`).
    pub fn from(&self) -> usize { self.from }
    /// Returns the upper bound of the range (`anchor.max(head)`).
    pub fn to(&self) -> usize { self.to }
    /// Returns the remembered visual goal column for vertical cursor movement, if any.
    pub fn column(&self) -> Option<usize> { self.column }
    /// Returns the side association of the cursor.
    pub fn assoc(&self) -> Assoc { self.assoc }
    /// Returns the fixed anchor offset of the selection.
    pub fn anchor(&self) -> usize { self.anchor }
    /// Returns the moving head (caret) offset of the selection.
    pub fn head(&self) -> usize { self.head }
    /// Returns `true` if `from == to` (i.e. this range is a cursor with no selected text).
    pub fn empty(&self) -> bool { self.from == self.to }

    /// Maps this selection range through a [`ChangeDesc`].
    ///
    /// - Empty cursors are mapped using the provided `assoc`.
    /// - Non-empty ranges map their `from` bound with [`Assoc::After`] and their `to` bound with
    ///   [`Assoc::Before`] so insertions at the edges do not expand the selection, while preserving
    ///   backward direction (`anchor > head`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::{Assoc, ChangeSet, ChangeSpec, SelectionRange};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let r = SelectionRange::range(2, 5);
    /// let cs = ChangeSet::from_slice(&[ChangeSpec::new(0, None, Some("XX".into()))?], 10)?;
    /// let mapped = r.map(cs.description(), Assoc::After)?;
    /// assert_eq!(mapped.from(), 4);
    /// assert_eq!(mapped.to(), 7);
    /// # Ok(())
    /// # }
    /// ```
    pub fn map(&self, change: &ChangeDesc, assoc: Assoc) -> Result<Self, Error> {
        let (new_from, new_to) = if self.empty() {
            let pos = change
                .map_pos(self.from, assoc, MapMode::Simple)?
                .unwrap_or(0);
            (pos, pos)
        } else {
            let f = change
                .map_pos(self.from, Assoc::After, MapMode::Simple)?
                .unwrap_or(0);
            let t = change
                .map_pos(self.to, Assoc::Before, MapMode::Simple)?
                .unwrap_or(0);
            (f, t.max(f))
        };

        let mut mapped = if new_from == new_to {
            Self::cursor(new_from)
        } else if self.anchor > self.head {
            Self::range(new_to, new_from)
        } else {
            Self::range(new_from, new_to)
        };
        mapped.assoc = self.assoc;
        mapped.column = None;
        Ok(mapped)
    }
    
    /// Extends the moving `head` of this selection to encompass `[from..to]`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::SelectionRange;
    ///
    /// let cur = SelectionRange::cursor(2).extend(5, Some(8));
    /// assert_eq!(cur.anchor(), 2);
    /// assert_eq!(cur.head(), 8);
    /// ```
    pub fn extend(mut self, from: usize, to: Option<usize>) -> Self {
        let to = to.unwrap_or(from);
        
        // If selecting backwards, extend the head to the start of the new range.
        // If selecting forwards, extend the head to the end of the new range.
        self.head = if self.head < self.anchor { from } else { to };
        
        // Recalculate absolute bounds
        self.from = std::cmp::min(self.anchor, self.head);
        self.to = std::cmp::max(self.anchor, self.head);
        
        // Invalidate visual column memory when selection changes
        self.column = None;
        
        self
    }
    
    /// Checks if this selection overlaps or touches another selection (assuming `self.from <= other.from`).
    pub fn overlaps(&self, other: &Self) -> bool {
        match self.empty() || other.empty() {
            true  => other.from <= self.to,
            false => other.from < self.to,
        }
    }
    
    /// Merges this selection with `other`, creating a bounding selection with `other`'s direction.
    pub fn merge(&mut self, other: &Self) {
        let new_from = self.from.min(other.from);
        let new_to = self.to.max(other.to);
        
        // Determine the new anchor and head based on the direction of the other selection.
        let (new_anchor, new_head) = match other.head < other.anchor {
            true  => (new_to, new_from),
            false => (new_from, new_to),
        };
        
        self.from = new_from;
        self.to = new_to;
        self.anchor = new_anchor;
        self.head = new_head;
    }
}

impl PartialEq for SelectionRange {
    fn eq(&self, other: &Self) -> bool {
        self.anchor == other.anchor && self.head == other.head
    }
}

impl Selection {
    /// Shorthand for [`SelectionRange::cursor`].
    pub fn cursor(pos: usize) -> SelectionRange {
        SelectionRange::cursor(pos)
    }
    
    /// Shorthand for [`SelectionRange::range`].
    pub fn range(anchor: usize, head: usize) -> SelectionRange {
        SelectionRange::range(anchor, head)
    }
}
