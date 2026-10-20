use mynox::Error;
use mynox::Formater;

use crate::Text;
use crate::errors::range_error;


use super::super::Composable;
use super::super::Mapable;
use super::super::ChangeSet;
use super::super::ChangeDesc;
use super::super::Section;
use super::super::ChangeSpec;
use super::super::utils::add_section_generic;


impl ChangeSet {
    pub(crate) fn new(sections: Vec<Section>, text: Vec<Text>) -> Self {
        assert_eq!(sections.len(), text.len());
        
        Self { 
            description: ChangeDesc { sections }, 
            insertions: text, 
        }
    }

    /// Compiles a slice of [`ChangeSpec`]s against a document of length `doc_len` into a
    /// normalized [`ChangeSet`].
    ///
    /// Out-of-order or overlapping [`ChangeSpec`]s in `specs` are automatically flushed, mapped,
    /// and composed in sequence. Returns a [`RangeError`](crate::ErrorTypes::RangeError) if any
    /// `spec.to > doc_len`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::{ChangeSet, ChangeSpec, Composable, Text};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let specs = [
    ///     ChangeSpec::new(0, Some(0), Some("start: ".into()))?,
    ///     ChangeSpec::new(4, Some(4), Some("!".into()))?,
    /// ];
    /// let cs = ChangeSet::from_slice(&specs, 4)?;
    /// assert_eq!(cs.len_old(), 4);
    /// assert_eq!(cs.len_new(), 12);
    /// assert_eq!(cs.apply(Text::from("test"))?.to_string(), "start: test!");
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_slice(specs: &[ChangeSpec], doc_len: usize) -> Result<Self, Error> {
        let mut sections: Vec<Section> = Vec::new();
        let mut inserts: Vec<Text> = Vec::new();
        let mut pos = 0;
        let mut total: Option<ChangeSet> = None;

        let flush = |
            sections: &mut Vec<Section>, 
            inserts: &mut Vec<Text>, 
            pos: &mut usize, 
            total: &mut Option<ChangeSet>, 
            force: bool
        | -> Result<(), Error> {
            if !force && sections.is_empty() { return Ok(()); }
            
            if *pos < doc_len {
                add_section_generic(
                    sections,
                    inserts,
                    doc_len - *pos,
                    Section::keep,
                    Text::new(),
                    false,
                );
            }
            
            let current = ChangeSet::new(std::mem::take(sections), std::mem::take(inserts));
            *total = match total.take() {
                Some(prev) => {
                    let mapped = current.map(&prev, false)?;
                    Some(prev.compose(&mapped)?)
                }
                None => Some(current),
            };
            *pos = 0;
            Ok(())
        };
        
        for spec in specs {
            if spec.to > doc_len {
                return Err(
                    range_error("The ChangeSpec range exceeds the document length")
                        .extend_note(format!("`to`: {}, `doc_len`: {}", spec.to, doc_len).as_tip_cotinuation())
                );
            }
            
            let ins_text = spec.insert.clone().unwrap_or_default();
            let ins_len = ins_text.len_chars();
            
            if spec.from == spec.to && ins_len == 0 { continue; }
            
            // If gets back respect to pos, close the current chunk
            if spec.from < pos { 
                flush(&mut sections, &mut inserts, &mut pos, &mut total, false)?; 
            }

            // If there's space before the change, we add a keep
            if spec.from > pos {
                add_section_generic(
                    &mut sections,
                    &mut inserts,
                    spec.from - pos,
                    Section::keep,
                    Text::new(),
                    false,
                );
            }

            // Add replace/insert/deletion section
            add_section_generic(
                &mut sections,
                &mut inserts,
                spec.to - spec.from,
                ins_len,
                ins_text,
                false,
            );
            
            pos = spec.to;
        }
        
        let is_empty_total = total.is_none();
        flush(&mut sections, &mut inserts, &mut pos, &mut total, is_empty_total)?;
        
        Ok(total.unwrap_or_else(|| {
            if doc_len > 0 { // Unchanged doc
                ChangeSet::new(
                    vec![Section { original: doc_len, replaced: Section::keep }],
                    vec![Text::new()],
                )
            } else {
                ChangeSet::new(Vec::new(), Vec::new())
            }
        }))
    }
}
