use crate::Text;

use super::ChangeDesc;

pub mod utils;
pub mod apply;
pub mod impls;
pub mod init;


/// A normalized, immutable sequence of document replacements that can be applied to a [`Text`]
/// document, composed with subsequent changes ([`Composable::compose`](crate::Composable::compose)),
/// mapped over concurrent changes ([`Mapable::map`](crate::Mapable::map)), or inverted for
/// undo/redo history ([`ChangeSet::invert`]).
///
/// # Examples
///
/// ```rust
/// use texera_core::{ChangeSet, ChangeSpec, Composable, Text};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let doc = Text::from("hello world");
/// let cs = ChangeSet::from_slice(
///     &[ChangeSpec::new(6, Some(11), Some(Text::from("rust")))?],
///     doc.len_chars(),
/// )?;
///
/// let updated = cs.apply(doc.clone())?;
/// assert_eq!(updated.to_string(), "hello rust");
///
/// // Invert the change set to undo the edit
/// let undo = cs.invert(&doc);
/// assert_eq!(undo.apply(updated)?.to_string(), "hello world");
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct ChangeSet {
    description: ChangeDesc,
    insertions: Vec<Text>,
}
