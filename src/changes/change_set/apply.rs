use mynox::Error;
use mynox::Formater;

use crate::changes::ChangeIterator;
use crate::errors::range_error;
use crate::Text;

use super::super::Composable;
use super::ChangeSet;


impl ChangeSet {
    /// Applies this [`ChangeSet`] to `text`, returning the modified [`Text`] document.
    ///
    /// Returns a [`RangeError`](crate::ErrorTypes::RangeError) if `self.len_old() != text.len_chars()`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::{ChangeSet, ChangeSpec, Text};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let doc = Text::from("abcdef");
    /// let cs = ChangeSet::from_slice(&[ChangeSpec::new(2, Some(4), Some("XY".into()))?], 6)?;
    /// assert_eq!(cs.apply(doc)?.to_string(), "abXYef");
    /// # Ok(())
    /// # }
    /// ```
    pub fn apply(&self, text: Text) -> Result<Text, Error> {
        if self.len_old() != text.len_chars() {
            return Err(
                range_error("Applying change set to a document with the wrong length")
                    .extend_note(format!("`set`: {}, `text`: {}", self.len_old(), text.len_chars()).as_tip_cotinuation())
            );
        }

        let mut rope = text.into_rope();
        let changes: Vec<_> = self.iter_changes(false).collect();
        for change in changes.into_iter().rev() {
            let from = change.map.old.from;
            let to = change.map.old.to;
            if to > from {
                rope.remove(from..to);
            }
            let mut insert_pos = from;
            for chunk in change.text.as_rope().chunks() {
                rope.insert(insert_pos, chunk);
                insert_pos += chunk.chars().count();
            }
        }

        Ok(Text::from_rope(rope))
    }
}
