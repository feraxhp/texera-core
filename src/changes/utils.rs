//! Internal accumulation and compression helpers for change sections and texts.

use crate::changes::{change_text::ChangeText, section::Section};

/// Appends a new section and text payload to accumulating buffers, compressing adjacent matching spans.
///
/// Adjoining keep sections, adjacent pure deletions, and contiguous insertions are merged
/// into single entries to minimize memory footprint and keep transformation algorithms efficient.
pub fn add_section_generic<T: ChangeText>(
    sections: &mut Vec<Section>,
    texts: &mut Vec<T>,
    original: usize,
    replaced: usize,
    text: T,
    force_join: bool,
) {
    if original == 0 && (replaced == 0 || replaced == Section::keep) {
        return;
    }

    if let Some(last) = sections.last_mut() {
        let last_idx = texts.len() - 1;

        if replaced == Section::keep && last.is_keep() {
            last.original += original;
            return;
        }
        if replaced == 0 && last.replaced == 0 && !last.is_keep() {
            last.original += original;
            return;
        }
        if original == 0 && last.original == 0 && !last.is_keep() {
            last.replaced += replaced;
            texts[last_idx].append(text);
            return;
        }
        if force_join {
            last.original += original;
            last.replaced += replaced;
            texts[last_idx].append(text);
            return;
        }
    }

    sections.push(Section { original, replaced });
    texts.push(text);
}
