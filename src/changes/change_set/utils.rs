use crate::Text;

use super::super::Composable;
use super::super::ChangeDesc;
use super::super::ChangeSet;


impl ChangeSet {
    /// Returns a reference to the underlying [`ChangeDesc`] of this change set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::{ChangeSet, ChangeSpec, Composable};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let cs = ChangeSet::from_slice(&[ChangeSpec::new(0, Some(2), None)?], 5)?;
    /// assert_eq!(cs.description().len_old(), 5);
    /// assert_eq!(cs.description().len_new(), 3);
    /// # Ok(())
    /// # }
    /// ```
    pub fn description(&self) -> &ChangeDesc { &self.description }
    
    /// Computes the inverse [`ChangeSet`] against the original document `text` (prior to applying `self`).
    ///
    /// Applying `self` to `text` and then applying `self.invert(&text)` restores `text` bit-for-bit.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::{ChangeSet, ChangeSpec, Text};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let original = Text::from("hello world");
    /// let cs = ChangeSet::from_slice(&[ChangeSpec::new(6, Some(11), Some("rust".into()))?], 11)?;
    /// let inverted = cs.invert(&original);
    ///
    /// let edited = cs.apply(original.clone())?;
    /// assert_eq!(edited.to_string(), "hello rust");
    /// assert_eq!(inverted.apply(edited)?.to_string(), "hello world");
    /// # Ok(())
    /// # }
    /// ```
    pub fn invert(&self, text: &Text) -> Self {
        let length = self.sections().len();
        let mut pos = 0;
        let mut insert = Vec::with_capacity(length);
        
        let sections = self.sections().iter()
            .map(|section| {
                let inverted = section.invert();
                
                if inverted.is_keep() || inverted.is_deletion() { insert.push(Text::new()); }
                else { insert.push(text.slice(pos, Some(pos + section.original))); }
                
                pos += section.original;
                inverted
            })
            .collect()
        ;
        
        Self::new(sections, insert)
    }
}
