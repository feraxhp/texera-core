use super::super::ChangeIterator;
use super::super::ChangeDesc;
use super::super::Composable;
use super::super::ChangeMap;
use super::super::Section;
use super::super::Mapable;
use super::super::Touched;
use super::super::Range;


impl ChangeDesc {
    /// Checks whether any modifications in this `ChangeDesc` intersect the range `[from..to]`.
    ///
    /// - If `to` is `None`, checks the point `[from..from]`.
    /// - Returns [`Touched::Cover`] if a single replaced/deleted section strictly encloses `[from..to]`.
    /// - Returns [`Touched::Yes`] if any change overlaps or touches `[from..to]`.
    /// - Returns [`Touched::No`] if `[from..to]` lies entirely within unchanged sections.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::{ChangeSet, ChangeSpec, Touched};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let cs = ChangeSet::from_slice(&[ChangeSpec::new(2, Some(6), Some("x".into()))?], 10)?;
    /// let desc = cs.description();
    ///
    /// assert_eq!(desc.touches_range(0, Some(1)), Touched::No);
    /// assert_eq!(desc.touches_range(1, Some(3)), Touched::Yes);
    /// assert_eq!(desc.touches_range(3, Some(5)), Touched::Cover);
    /// # Ok(())
    /// # }
    /// ```
    pub fn touches_range(&self, from: usize, to: Option<usize>) -> Touched {
        let to = to.unwrap_or(from);
        let mut pos = 0;
        let mut touched = false;

        for section in &self.sections {
            let end = pos + section.original;
            if !section.is_keep() {
                if pos <= to && end >= from {
                    if pos < from && end > to {
                        return Touched::Cover;
                    }
                    touched = true;
                }
            }
            pos = end;
            if pos > to {
                break;
            }
        }

        if touched { Touched::Yes } else { Touched::No }
    }
}

impl Mapable for ChangeDesc { }
impl Composable for ChangeDesc {
    type Text = ();
    
    fn sections(&self) -> &[Section] { &self.sections }
    fn len_old(&self) -> usize {
        self.sections.iter()
            .fold(0, |acc, s| acc + s.original)
    }
    
    fn len_new(&self) -> usize {
        self.sections.iter()
            .fold(0, |acc, s| {
                match s.is_keep() {
                    true  => acc + s.original,
                    false => acc + s.replaced,
                }
            })
    }
    
    fn is_empty(&self) -> bool {
        let length = self.sections.len();
        match length {
            0 => true,
            1 => unsafe { self.sections.get_unchecked(0).is_keep() },
            _ => false,
        }
    }
    
    fn text_slice(&self, _: usize, _: usize, _: usize) -> Self::Text { () }
    fn text_at(&self, _: usize) -> Self::Text { () }
    fn from_composed(sections: Vec<Section>, _: Vec<Self::Text>) -> Self { Self { sections } }
    
}

impl ChangeIterator for ChangeDesc {
    type Item = ChangeMap<Range>;
    
    fn get_change_iterator_item(&self, _: Range, map: ChangeMap<Range>) -> Self::Item { map }
}
