use super::ChangeDesc;


impl ChangeDesc {
    /// Returns the inverse [`ChangeDesc`], swapping original and replaced section lengths.
    ///
    /// Composing a `ChangeDesc` with its inverse yields an identity description of length `len_old()`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::{ChangeSet, ChangeSpec, Composable};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let cs = ChangeSet::from_slice(&[ChangeSpec::new(1, Some(3), Some("abcd".into()))?], 5)?;
    /// let desc = cs.description();
    /// let inv = desc.invert();
    ///
    /// assert_eq!(inv.len_old(), desc.len_new());
    /// assert_eq!(inv.len_new(), desc.len_old());
    /// # Ok(())
    /// # }
    /// ```
    pub fn invert(&self) -> Self {
        let sections = self.sections.iter()
                .map(|sec| sec.invert() )
                .collect()
        ;
        
        Self { sections }
    }
}
