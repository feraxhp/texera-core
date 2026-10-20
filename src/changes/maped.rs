use crate::changes::composed::Composable;
use crate::changes::section::Section;
use crate::changes::section_iter::SectionIter;
use crate::changes::utils::add_section_generic;
use crate::errors::differing_changes;
use mynox::{Error, Formater};

/// Operational transformation (rebasing) trait for [`ChangeDesc`](crate::ChangeDesc) and
/// [`ChangeSet`](crate::ChangeSet).
///
/// Given two changes `A` (`self`) and `B` (`other`) that both start from the **same** base document
/// (`A.len_old() == B.len_old()`), `A.map(&B, before)` transforms `A` so that it can be applied
/// **after** `B` (`B.compose(&A.map(&B, before))`).
///
/// # Examples
///
/// ```rust
/// use texera_core::{ChangeSet, ChangeSpec, Composable, Mapable, Text};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let doc = Text::from("abcd");
/// // Both changes are created against "abcd" (len = 4)
/// let a = ChangeSet::from_slice(&[ChangeSpec::new(2, None, Some("X".into()))?], 4)?;
/// let b = ChangeSet::from_slice(&[ChangeSpec::new(0, None, Some("YY".into()))?], 4)?;
///
/// // Map `a` over `b` so `a` can be composed after `b`
/// let a_after_b = a.map(&b, false)?;
/// let combined = b.compose(&a_after_b)?;
/// assert_eq!(combined.apply(doc)?.to_string(), "YYabXcd");
/// # Ok(())
/// # }
/// ```
pub trait Mapable: Composable {
    /// Rebases `self` over concurrent change `other` that started from the same document length.
    ///
    /// `before` controls tie-breaking when both changes insert at the exact same position:
    /// if `true`, `self`'s insertion is placed before `other`'s insertion.
    fn map<O: Composable>(&self, other: &O, before: bool) -> Result<Self, Error> {
        if self.len_old() != other.len_old() {
            return Err(
                differing_changes("Imposible to map this to changes")
                    .extend_detail("The **other change** `orginal length` should be".as_tip())
                    .extend_detail("equal to **self** `orginal length`".as_tip_cotinuation())
                    .extend_note("This is because **other** should start")
                    .extend_note("in the same document as the **self**.")
                    .extend_note("")
                    .extend_note(format!("`{}` != `{}`", self.len_old(), other.len_old()).as_tip())
            );
        }

        if other.is_empty() { return Ok(self.clone()); }

        let cap = self.sections().len().max(other.sections().len());
        let mut sections = Vec::with_capacity(cap);
        let mut texts    = Vec::with_capacity(cap);

        let mut iter_a   = SectionIter::new(self.sections());
        let mut iter_b   = SectionIter::new(other.sections());
        let mut inserted: isize = -1;

        loop {
            let a_done = iter_a.is_done();
            let b_done = iter_b.is_done();
            let a_len  = iter_a.remaining_original();
            let b_len  = iter_b.remaining_original();

            if a_done && b_done { 
                return Ok(Self::from_composed(sections, texts)); 
            }
            else if (a_done && b_len > 0) || (b_done && a_len > 0) {
                return Err(
                    differing_changes("Imposible to map this to changes")
                        .extend_detail("The **other change** `orginal length` should be".as_tip())
                        .extend_detail("equal to **self** `orginal length`".as_tip_cotinuation())
                        .extend_note("This is because **other** should start")
                        .extend_note("in the same document as the **self**.")
                );
            }
            else if iter_a.is_keep() && iter_b.is_keep() {
                let len = a_len.min(b_len);

                add_section_generic(
                    &mut sections, &mut texts, 
                    len, Section::keep, 
                    Self::Text::default(), false
                );
                iter_a.forward_original(len);
                iter_b.forward_original(len);
            }
            else if !b_done && !iter_b.is_keep() && (
                a_done
                || iter_a.is_keep()
                || inserted == iter_a.index() as isize
                || (iter_a.offset() == 0 && (b_len < a_len || (b_len == a_len && !before)))
            ) {
                let mut len = b_len;
                add_section_generic(
                    &mut sections, &mut texts, 
                    iter_b.current().replaced, Section::keep, 
                    Self::Text::default(), false
                );

                while len > 0 {
                    if iter_a.is_done() {
                        return Err(
                            differing_changes("Imposible to map this to changes")
                                .extend_detail("The **other change** `orginal length` should be".as_tip())
                                .extend_detail("equal to **self** `orginal length`".as_tip_cotinuation())
                                .extend_note("This is because **other** should start")
                                .extend_note("in the same document as the **self**.")
                        );
                    }

                    let piece = iter_a.remaining_original().min(len);
                    if !iter_a.is_keep()
                        && inserted < iter_a.index() as isize
                        && iter_a.remaining_original() <= piece
                    {
                        let text = self.text_at(iter_a.index());
                        add_section_generic(
                            &mut sections, &mut texts, 
                            0, iter_a.current().replaced, 
                            text, false
                        );
                        inserted = iter_a.index() as isize;
                    }

                    iter_a.forward_original(piece);
                    len -= piece;
                }
                iter_b.next();
            }
            else if !a_done && !iter_a.is_keep() {
                let mut len  = 0;
                let mut left = a_len;

                while left > 0 {
                    if iter_b.is_keep() {
                        let piece = left.min(iter_b.remaining_original());

                        len  += piece;
                        left -= piece;
                        iter_b.forward_original(piece);
                    }
                    else if !iter_b.is_done() && iter_b.current().is_deletion() && iter_b.remaining_original() < left {
                        left -= iter_b.remaining_original();
                        iter_b.next();
                    }
                    else { break; }
                }

                let (repl, text) = if inserted < iter_a.index() as isize {
                    (iter_a.current().replaced, self.text_at(iter_a.index()))
                } else { 
                    (0, Self::Text::default()) 
                };

                add_section_generic(
                    &mut sections, &mut texts, 
                    len, repl, 
                    text, false
                );
                inserted = iter_a.index() as isize;
                iter_a.forward_original(a_len - left);
            }
            else {
                return Err(
                    differing_changes("Imposible to map this to changes")
                        .extend_detail("The **other change** `orginal length` should be".as_tip())
                        .extend_detail("equal to **self** `orginal length`".as_tip_cotinuation())
                        .extend_note("This is because **other** should start")
                        .extend_note("in the same document as the **self**.")
                );
            }
        }
    }
}
