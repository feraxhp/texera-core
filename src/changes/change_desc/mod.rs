use super::Section;

pub mod utils;
pub mod impls;


/// A lightweight description of document changes that stores only section lengths (retained,
/// deleted, and inserted character counts) without retaining the inserted text.
///
/// `ChangeDesc` is used to efficiently map positions ([`ChangeDesc::map_pos`]), selections
/// ([`Selection::map`](crate::Selection::map)), and range sets ([`RangeSet::map`](crate::RangeSet::map))
/// through document edits, and implements [`Composable`](crate::Composable), [`Mapable`](crate::Mapable),
/// and [`ChangeIterator`](crate::ChangeIterator).
///
/// # Examples
///
/// ```rust
/// use texera_core::{Assoc, ChangeSet, ChangeSpec, Composable, MapMode};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let cs = ChangeSet::from_slice(&[ChangeSpec::new(2, Some(4), Some("hello".into()))?], 6)?;
/// let desc = cs.description();
///
/// assert_eq!(desc.len_old(), 6);
/// assert_eq!(desc.len_new(), 9);
/// assert_eq!(desc.map_pos(5, Assoc::After, MapMode::Simple)?, Some(8));
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeDesc {
    pub(crate) sections: Vec<Section>,
}
