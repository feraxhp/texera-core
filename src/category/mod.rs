//! Character classification, word boundaries, grapheme clusters, and column calculations.
//!
//! This module provides Unicode-aware utilities used by cursor movement, word
//! selection ([`State::word_at`](crate::State::word_at)), and visual column alignment:
//!
//! - [`CharCategory`] and [`CharCategorizer`] — classify characters or strings into
//!   [`Word`](CharCategory::Word), [`Space`](CharCategory::Space), or [`Other`](CharCategory::Other).
//! - [`WordChars`] — a built-in facet for registering extra language-specific characters
//!   (like `$` or `-`) as word characters.
//! - [`find_cluster_break`] — step across Unicode extended grapheme clusters.
//! - [`count_column`] and [`find_column`] — convert between character offsets and visual
//!   columns taking tab stops and CJK/emoji widths into account.

pub mod categorizer;
pub mod unicode;
mod structs;
mod tests;

pub use self::categorizer::make_categorizer;
pub use self::categorizer::CategorizeInput;
pub use self::unicode::find_cluster_break;
pub use self::unicode::count_column;
pub use self::unicode::find_column;
pub use self::structs::*;
