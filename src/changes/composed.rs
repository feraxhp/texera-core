use crate::changes::{change_text::ChangeText, utils::add_section_generic};
use crate::changes::section::Section;
use crate::changes::section_iter::SectionIter;
use crate::errors::differing_changes;
use mynox::{Error, Formater};


/// Shared interface for sequentially composable change structures ([`ChangeDesc`](crate::ChangeDesc)
/// and [`ChangeSet`](crate::ChangeSet)).
///
/// If change `A` transforms a document of length $L_0 \to L_1$ and change `B` transforms
/// $L_1 \to L_2$, then `A.compose(&B)` produces a single change transforming $L_0 \to L_2$ with
/// the exact same effect as applying `A` followed by `B`.
///
/// # Examples
///
/// ```rust
/// use texera_core::{ChangeSet, ChangeSpec, Composable, Text};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let doc = Text::from("abc");
/// let cs1 = ChangeSet::from_slice(&[ChangeSpec::new(3, None, Some("d".into()))?], 3)?;
/// let cs2 = ChangeSet::from_slice(&[ChangeSpec::new(0, Some(1), Some("A".into()))?], 4)?;
///
/// let composed = cs1.compose(&cs2)?;
/// assert_eq!(composed.len_old(), 3);
/// assert_eq!(composed.len_new(), 4);
/// assert_eq!(composed.apply(doc)?.to_string(), "Abcd");
/// # Ok(())
/// # }
/// ```
pub trait Composable: Sized + Clone {
    #[doc(hidden)]
    type Text: ChangeText;
    
    #[doc(hidden)]
    fn sections(&self) -> &[Section];
    /// Returns the expected character length of the document before this change.
    fn len_old(&self) -> usize;
    /// Returns the character length of the document after applying this change.
    fn len_new(&self) -> usize;
    /// Returns `true` if this change performs no insertions, deletions, or replacements.
    fn is_empty(&self) -> bool;
    #[doc(hidden)]
    fn text_slice(&self, section_idx: usize, offset: usize, len: usize) -> Self::Text;
    #[doc(hidden)]
    fn text_at(&self, section_idx: usize) -> Self::Text;
    #[doc(hidden)]
    fn from_composed(sections: Vec<Section>, texts: Vec<Self::Text>) -> Self;
    
    /// Sequentially composes `self` with `other` (`self` followed by `other`).
    ///
    /// Returns a [`DifferingChanges`](crate::ErrorTypes::DifferingChanges) error if
    /// `self.len_new() != other.len_old()`.
    fn compose(&self, other: &Self) -> Result<Self, Error> {
        if self.len_new() != other.len_old() {
            return Err(
                differing_changes("Impossible to compose these two changes")
                    .extend_detail("The **other change** `orginal length` should be".as_tip())
                    .extend_detail("equal to **self** `produced length`".as_tip_cotinuation())
                    .extend_note(format!("`{}` != `{}`", self.len_new(), other.len_old()).as_tip())
            );
        }
        
        if self.is_empty()  { return Ok(other.clone()); }
        if other.is_empty() { return Ok(self.clone());  }
        
        let cap = self.sections().len().max(other.sections().len());
        let mut new_sections = Vec::with_capacity(cap);
        let mut new_texts    = Vec::with_capacity(cap);
        
        let mut iter_origin = SectionIter::new(self.sections());
        let mut iter_next   = SectionIter::new(other.sections());
        let mut open        = false;
        
        loop {
            if iter_origin.is_done() && iter_next.is_done() {
                return Ok(Self::from_composed(new_sections, new_texts));
            }
            else if !iter_origin.is_done() && iter_origin.current().is_deletion() {
                add_section_generic(
                    &mut new_sections, &mut new_texts, 
                    iter_origin.current().original, 0, 
                    Self::Text::default(), open
                );
                iter_origin.next();
            }
            else if !iter_next.is_done() && iter_next.current().is_insertion() {
                let text = other.text_at(iter_next.index());
                add_section_generic(
                    &mut new_sections, &mut new_texts, 
                    0, iter_next.current().replaced, 
                    text, open
                );
                iter_next.next();
            }
            else if iter_origin.is_done() || iter_next.is_done() {
                return Err(differing_changes("Imposible to compose this to changes"));
            }
            else {
                let len_origin  = iter_origin.remaining_on_intermediate();
                let len_next    = iter_next.remaining_original();
                let len         = len_origin.min(len_next);
                let section_len = new_sections.len();
                
                let origin_is_keep = iter_origin.current().is_keep();
                let next_is_keep   = iter_next.current().is_keep();
                
                if origin_is_keep {
                    let ins_next = if next_is_keep { 
                        Section::keep 
                    } else if iter_next.offset() > 0 { 
                        0 
                    } else { 
                        iter_next.current().replaced 
                    };
                    
                    let text = if ins_next == Section::keep || ins_next == 0 {
                        Self::Text::default()
                    } else {
                        other.text_at(iter_next.index())
                    };
                    
                    add_section_generic(&mut new_sections, &mut new_texts, len, ins_next, text, open);
                }
                else if next_is_keep {
                    let orig_origin = if iter_origin.offset() > 0 { 0 } else { iter_origin.current().original };
                    let text = self.text_slice(iter_origin.index(), iter_origin.offset(), len);

                    add_section_generic(&mut new_sections, &mut new_texts, orig_origin, len, text, open);
                }
                else {
                    let orig_origin = if iter_origin.offset() > 0 { 0 } else { iter_origin.current().original };
                    let repl_next   = if iter_next.offset() > 0 { 0 } else { iter_next.current().replaced };
                    let text = if repl_next > 0 && iter_next.offset() == 0 {
                        other.text_at(iter_next.index())
                    } else {
                        Self::Text::default()
                    };
                    
                    add_section_generic(&mut new_sections, &mut new_texts, orig_origin, repl_next, text, open);
                }
                
                let origin_has_more = !origin_is_keep && iter_origin.remaining_on_intermediate() > len;
                let next_has_more   = !next_is_keep && iter_next.remaining_original() > len;
                open = (origin_has_more || next_has_more) && (open || new_sections.len() > section_len);
                
                iter_origin.forward_intermediate(len);
                iter_next.forward_original(len);
            }
        }
    }
}
