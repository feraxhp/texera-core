use mynox::Error;
use mynox::Formater;

use crate::changes::ChangeDesc;
use crate::errors::range_error;

use super::Assoc;
use super::Selection;
use super::SelectionSpec;
use super::SelectionRange;

impl Default for Selection {
    fn default() -> Self {
        Self::new()
    }
}

impl Selection {
    /// Maps all ranges in this selection through a [`ChangeDesc`], re-normalizing and merging
    /// any ranges that collapse onto each other.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::{Assoc, ChangeSet, ChangeSpec, Selection};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let sel = Selection::from([Selection::cursor(0), Selection::cursor(4)]);
    /// let cs = ChangeSet::from_slice(&[ChangeSpec::new(0, None, Some("ab".into()))?], 6)?;
    /// let mapped = sel.map(cs.description(), Assoc::After)?;
    /// assert_eq!(mapped.ranges_ref()[0].from(), 2);
    /// assert_eq!(mapped.ranges_ref()[1].from(), 6);
    /// # Ok(())
    /// # }
    /// ```
    pub fn map(&self, change: &ChangeDesc, assoc: Assoc) -> Result<Selection, Error> {
        let mut mapped_ranges = Vec::with_capacity(self.ranges.len());
        for range in &self.ranges {
            mapped_ranges.push(range.map(change, assoc)?);
        }
        Self::from_with_head(mapped_ranges, self.main)
    }

    /// Creates a default [`Selection`] with a single cursor at offset `0`.
    pub fn new() -> Self {
        Selection { 
            ranges: vec![SelectionRange::cursor(0)], 
            main: 0 
        }
    }
    
    fn merge_ranges_with_main<V>(
        selections: V,
        main_idx: Option<usize>,
    ) -> Result<(Vec<SelectionRange>, usize), Error>
    where 
        V: IntoIterator,
        V::Item: Into<SelectionRange>,
    {
        let raw: Vec<SelectionRange> = selections.into_iter().map(Into::into).collect();
        let raw_len = raw.len();
        let target_main = match main_idx {
            Some(m) => {
                if m >= raw_len {
                    return Err(
                        range_error("**main** should be less than **selections len**")
                            .extend_note(format!("main: `{m}`, length: `{raw_len}`").as_tip_cotinuation())
                    );
                }
                m
            }
            None => raw_len.saturating_sub(1),
        };

        let mut indexed: Vec<(SelectionRange, bool)> = raw
            .into_iter()
            .enumerate()
            .map(|(i, r)| (r, i == target_main))
            .collect();
        indexed.sort_by_key(|(s, _)| (s.from(), s.to()));
        
        let mut sel: Vec<SelectionRange> = Vec::with_capacity(indexed.len());
        let mut final_main = 0;
        
        for (range, is_main) in indexed {
            if let Some(last) = sel.last_mut()
                && last.overlaps(&range)
            {
                last.merge(&range);
                if is_main {
                    final_main = sel.len() - 1;
                }
            } else {
                if is_main {
                    final_main = sel.len();
                }
                sel.push(range);
            }
        }

        if main_idx.is_none() {
            final_main = sel.len().saturating_sub(1);
        }
        
        Ok((sel, final_main))
    }
    
    /// Constructs a normalized [`Selection`] from an iterator of [`SelectionRange`]s.
    ///
    /// Overlapping ranges are merged and sorted by position.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::Selection;
    ///
    /// let sel = Selection::from([
    ///     Selection::range(0, 4),
    ///     Selection::range(2, 6),
    ///     Selection::cursor(10),
    /// ]);
    /// assert_eq!(sel.count(), 2);
    /// assert_eq!(sel.ranges_ref()[0], Selection::range(0, 6));
    /// ```
    pub fn from<V>(selection: V) -> Selection
    where 
        V: IntoIterator,
        V::Item: Into<SelectionRange>,
    {
        let (ranges, main) = Self::merge_ranges_with_main(selection, None)
            .expect("merge_ranges_with_main with None never fails");
        
        Self { ranges, main }
    }
    
    /// Constructs a normalized [`Selection`] with an explicit primary range index `main`.
    ///
    /// Returns a [`RangeError`](crate::ErrorTypes::RangeError) if `main` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::Selection;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let sel = Selection::from_with_head(
    ///     [Selection::cursor(0), Selection::cursor(5), Selection::cursor(10)],
    ///     1,
    /// )?;
    /// assert_eq!(sel.main().from(), 5);
    /// assert_eq!(sel.main_index(), 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_with_head<V>(selection: V, main: usize) -> Result<Selection, Error>
    where 
        V: IntoIterator,
        V::Item: Into<SelectionRange>,
    {
        let (ranges, main) = Self::merge_ranges_with_main(selection, Some(main))?;
        Ok(Selection { ranges, main })
    }
    
    /// Returns a reference to the primary (`main`) [`SelectionRange`].
    pub fn main<'a>(&'a self) -> &'a SelectionRange { 
        self.ranges.get(self.main).unwrap()
    }

    /// Returns the index of the primary (`main`) range within [`Selection::ranges_ref`].
    pub fn main_index(&self) -> usize {
        self.main
    }
    
    /// Collapses a multi-range selection down to a single-range [`Selection`] containing only [`Selection::main`].
    pub fn as_single(self) -> Self { 
        let range = self.main();
        
        Self { ranges: vec![range.to_owned()], main: 0 }
    }
    
    /// Adds `range` to the selection, re-normalizing and optionally making `range` the new `main` range.
    pub fn add_range(mut self, range: SelectionRange, main: bool) -> Selection { 
        let new_main = if main { self.ranges.len() } else { self.main };
        self.ranges.push(range);
        Self::from_with_head(self.ranges, new_main).unwrap_or_default()
    }
    
    /// Replaces the range at index `which` with `range` and re-normalizes the selection.
    pub fn replace_range(mut self, range: SelectionRange, which: usize) -> Selection { 
        if which < self.ranges.len() {
            self.ranges[which] = range;
        }
        Self::from_with_head(self.ranges, self.main).unwrap_or_default()
    }
    
    /// Returns the number of ranges in this selection.
    pub fn count(&self) -> usize { self.ranges.len() }
    /// Consumes `self` and returns the underlying vector of [`SelectionRange`]s.
    pub fn ranges(self) -> Vec<SelectionRange> { self.ranges }
    /// Returns a slice of all [`SelectionRange`]s in this selection.
    pub fn ranges_ref<'a>(&'a self) -> &'a [SelectionRange] { &self.ranges }

    /// Iterates by reference over `(range, is_main)` pairs without cloning or heap allocation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::Selection;
    ///
    /// let sel = Selection::from([Selection::cursor(1), Selection::cursor(4)]);
    /// let heads: Vec<(usize, bool)> = sel.iter().map(|(r, m)| (r.head(), m)).collect();
    /// assert_eq!(heads, vec![(1, false), (4, true)]);
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = (&SelectionRange, bool)> {
        let main = self.main;
        self.ranges
            .iter()
            .enumerate()
            .map(move |(idx, sel)| (sel, idx == main))
    }
}

impl PartialEq for Selection {
    fn eq(&self, other: &Self) -> bool {
        self.ranges == other.ranges && self.main == other.main
    }
}

/// Zero-allocation owning iterator over `(SelectionRange, bool)` pairs in a [`Selection`].
pub struct SelectionIntoIter {
    iter: std::iter::Enumerate<std::vec::IntoIter<SelectionRange>>,
    main: usize,
}

impl Iterator for SelectionIntoIter {
    type Item = (SelectionRange, bool);

    fn next(&mut self) -> Option<Self::Item> {
        let (idx, sel) = self.iter.next()?;
        Some((sel, idx == self.main))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl ExactSizeIterator for SelectionIntoIter {}

impl IntoIterator for Selection {
    type Item = (SelectionRange, bool);
    type IntoIter = SelectionIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        SelectionIntoIter {
            iter: self.ranges.into_iter().enumerate(),
            main: self.main,
        }
    }
}

impl FromIterator<SelectionRange> for Selection {
    fn from_iter<I: IntoIterator<Item = SelectionRange>>(iter: I) -> Self {
        let iter = iter.into_iter();
        
        Selection::from(iter)
    }
}

impl FromIterator<(SelectionRange, bool)> for Selection {
    fn from_iter<I: IntoIterator<Item = (SelectionRange, bool)>>(iter: I) -> Self {
        let iter = iter.into_iter();
        
        let (lower, _) = iter.size_hint();
        let mut ranges = Vec::with_capacity(lower);
        let mut main = 0;

        for (idx, (range, is_main)) in iter.enumerate() {
            if is_main { main = idx; }
            ranges.push(range);
        }

        Selection { ranges, main }
    }
}

impl FromIterator<SelectionRange> for SelectionSpec {
    fn from_iter<T: IntoIterator<Item = SelectionRange>>(iter: T) -> Self {
        let selection = Selection::from(iter);
        
        SelectionSpec::Selection(selection)
    }
}

impl From<Selection> for SelectionSpec {
    fn from(value: Selection) -> Self {
        SelectionSpec::Selection(value)
    }
}

impl From<SelectionRange> for SelectionSpec {
    fn from(value: SelectionRange) -> Self {
        SelectionSpec::Selection(value.into())
    }
}

impl From<SelectionRange> for Selection {
    fn from(value: SelectionRange) -> Self {
        Selection::from([value])
    }
}
