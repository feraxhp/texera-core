use super::SimpleChange;
use super::Composable;
use super::ChangeMap;
use super::Position;
use super::Range;

macro_rules! map { ($old:expr => $new:expr) => { ChangeMap { old: $old, new: $new } }; }
macro_rules! range { ($from:expr, $to:expr) => { Range     { from: $from, to: $to } }; }


/// Trait for iterating over modified and unmodified sections of a [`ChangeDesc`](crate::ChangeDesc)
/// or [`ChangeSet`](crate::ChangeSet).
///
/// # Examples
///
/// ```rust
/// use texera_core::{ChangeIterator, ChangeSet, ChangeSpec};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let cs = ChangeSet::from_slice(&[ChangeSpec::new(2, Some(4), Some("XYZ".into()))?], 6)?;
///
/// let changes: Vec<_> = cs
///     .iter_changes(false)
///     .map(|c| (c.map.old.from, c.map.old.to, c.map.new.from, c.map.new.to, c.text.to_string()))
///     .collect();
/// assert_eq!(changes, vec![(2, 4, 2, 5, "XYZ".to_string())]);
///
/// let unchanged: Vec<_> = cs
///     .iter_unchanged()
///     .map(|u| (u.map.old, u.map.new, u.length))
///     .collect();
/// assert_eq!(unchanged, vec![(0, 0, 2), (4, 5, 2)]);
/// # Ok(())
/// # }
/// ```
pub trait ChangeIterator: Composable {
    /// Item yielded by [`ChangeIterator::iter_changes`].
    type Item;
    
    #[doc(hidden)]
    fn get_change_iterator_item(&self, sections: Range, map: ChangeMap<Range>) -> Self::Item;

    /// Iterates over all modified ranges in the change set or description.
    ///
    /// If `individual` is `false`, adjacent replacement sections are merged into a single item.
    fn iter_changes(&self, individual: bool) -> impl Iterator<Item = Self::Item> {
        let mut sections = self.sections()
            .iter()
            .enumerate()
            .peekable();
        
        let mut pos_a = 0;
        let mut pos_b = 0;
        
        std::iter::from_fn(move || {
            // Skipt keeps
            while let Some((_, s)) = sections.peek() {
                if s.is_keep() {
                    pos_a += s.original;
                    pos_b += s.original;
                    sections.next();
                } else {
                    break;
                }
            }
            
            let (start_idx, first) = sections.next()?;
            let mut end_idx = start_idx + 1;
            
            let from_a = pos_a;
            let from_b = pos_b;
            
            let mut to_a = pos_a + first.original;
            let mut to_b = pos_b + first.replaced;
            
            if !individual {
                while let Some((_, next)) = sections.peek() {
                    if next.is_keep() { break; }
                    let (idx_, s) = sections.next().unwrap();
                    
                    to_a += s.original;
                    to_b += s.replaced;
                    end_idx = idx_ + 1;
                }
            }
            
            pos_a = to_a;
            pos_b = to_b;
            
            Some(self.get_change_iterator_item(
                range!(start_idx, end_idx),
                map!(range!(from_a, to_a) => range!(from_b, to_b)),
            ))
        })
    }

    /// Iterates over all unchanged ("keep") spans between modifications.
    fn iter_unchanged(&self) -> impl Iterator<Item = SimpleChange<Position>> {
        let mut old = 0;
        let mut new = 0;
        
        self.sections().iter()
            .filter_map(move |s| {
                let old_ = old;
                old += s.original;
                
                if s.is_keep() {
                    let position = map!(old_ => new);
                    new += s.original;
                    Some(SimpleChange { length: s.original, map: position })
                }
                else {
                    new += s.replaced;
                    None
                }
            })
    }
}
