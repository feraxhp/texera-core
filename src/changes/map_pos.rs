use mynox::Error;

use crate::selection::Assoc;
use crate::errors::out_of_bounds;

use super::ChangeDesc;

/// Controls how [`ChangeDesc::map_pos`] behaves when a position falls inside or on the boundary
/// of a deleted/replaced range.
///
/// # Examples
///
/// ```rust
/// use texera_core::{Assoc, ChangeSet, ChangeSpec, MapMode};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Replace range [2..6] with "XY" in a document of length 10
/// let cs = ChangeSet::from_slice(&[ChangeSpec::new(2, Some(6), Some("XY".into()))?], 10)?;
/// let desc = cs.description();
///
/// // Position 4 is strictly inside the deleted range [2..6]:
/// assert_eq!(desc.map_pos(4, Assoc::Before, MapMode::Simple)?, Some(2));
/// assert_eq!(desc.map_pos(4, Assoc::Before, MapMode::TrackDel)?, None);
///
/// // Position 2 is on the left boundary of [2..6]:
/// assert_eq!(desc.map_pos(2, Assoc::Before, MapMode::TrackAfter)?, None);
/// assert_eq!(desc.map_pos(2, Assoc::Before, MapMode::TrackBefore)?, Some(2));
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapMode {
    /// Always returns `Some(new_pos)`, clamping deleted positions to one end of the replacement
    /// according to [`Assoc`].
    Simple,
    /// Returns `None` if the position is strictly inside a deleted/replaced range (`from < pos < to`).
    TrackDel,
    /// Returns `None` if the character immediately **before** `pos` was deleted (`from < pos <= to`).
    TrackBefore,
    /// Returns `None` if the character immediately **after** `pos` was deleted (`from <= pos < to`).
    TrackAfter,
}

impl MapMode {
    #[inline]
    fn is_simple(&self) -> bool { matches!(self, Self::Simple) }
    #[inline]
    fn is_track_del(&self) -> bool { matches!(self, Self::TrackDel) }
    #[inline]
    fn is_track_before(&self) -> bool { matches!(self, Self::TrackBefore) }
    #[inline]
    fn is_track_after(&self) -> bool { matches!(self, Self::TrackAfter) }
}

impl ChangeDesc {
    /// Maps a character offset `pos` from the old document to the new document.
    ///
    /// - `assoc`: Determines whether `pos` sticks to the character [`Assoc::Before`] or
    ///   [`Assoc::After`] when text is inserted at `pos`.
    /// - `mode`: Determines whether deletions covering `pos` cause `Ok(None)` to be returned.
    ///
    /// Returns an [`OutOfBounds`](crate::ErrorTypes::OutOfBounds) error if `pos > self.len_old()`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::{Assoc, ChangeSet, ChangeSpec, MapMode};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let cs = ChangeSet::from_slice(&[ChangeSpec::new(2, Some(2), Some("ABC".into()))?], 5)?;
    /// let desc = cs.description();
    ///
    /// assert_eq!(desc.map_pos(2, Assoc::Before, MapMode::Simple)?, Some(2));
    /// assert_eq!(desc.map_pos(2, Assoc::After, MapMode::Simple)?, Some(5));
    /// # Ok(())
    /// # }
    /// ```
    pub fn map_pos(&self, pos: usize, assoc: Assoc, mode: MapMode) -> Result<Option<usize>, Error> {
        let mut original_pos = 0;
        let mut target_pos = 0;
        
        for section in &self.sections {
            let original_end = original_pos + section.original;
            if section.is_keep() { 
                if original_end > pos { return Ok(Some(target_pos + (pos - original_pos))); }
                target_pos += section.original;
            }
            else {
                if !mode.is_simple() 
                    && original_end >= pos 
                    && (
                          mode.is_track_del() && original_pos < pos && original_end > pos 
                          || mode.is_track_before() && original_pos < pos 
                          || mode.is_track_after() && original_end > pos 
                ) { return Ok(None); }
                
                if original_end > pos 
                || (
                    original_end == pos 
                    && assoc.is_before() 
                    && section.original == 0
                ) { 
                    return Ok(match pos == original_pos ||assoc.is_before()  {
                        true => Some(target_pos),
                        false => Some(target_pos + section.replaced),
                    })
                }
                
                target_pos += section.replaced
            }
            
            original_pos = original_end
        }
        
        if pos > original_pos { return Err(out_of_bounds(pos, original_pos)); }
        
        return Ok(Some(target_pos));
    }
}
