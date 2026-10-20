//! Text change descriptions, change sets, operational transformation, and position mapping.
//!
//! This module implements the core change tracking and operational transformation engine
//! ported from `@codemirror/state`:
//!
//! - [`ChangeSpec`]: Declarative description of an atomic text insertion, deletion, or replacement.
//! - [`ChangeDesc`]: Lightweight character-count summary of changes (without inserted text),
//!   used for mapping coordinates and computing document diffs.
//! - [`ChangeSet`]: Full collection of changes including inserted [`Text`](crate::Text) ropes,
//!   which can be applied to a document via [`ChangeSet::apply`].
//! - [`Composable`]: Trait for sequentially combining changes across successive versions of a document.
//! - [`Mapable`]: Trait for operational transformation (mapping one change over another applied concurrently).
//! - [`MapMode`]: Rules specifying how boundary positions map across insertions or deletions.
//! - [`ch!`]: Convenience macro for declaratively constructing [`ChangeSpec`] instances.

pub mod change_spec;
pub mod change_desc;
pub mod change_set;
mod section_iter;
pub mod iterator;
pub mod composed;
mod change_text;
pub mod map_pos;
pub mod maped;
mod structs;
mod section;
mod macros;
mod utils;
mod tests;


pub(crate) use self::section::Section;
pub use self::structs::*;

pub use self::change_desc::ChangeDesc;
pub use self::change_spec::ChangeSpec;
pub use self::change_set::ChangeSet;

pub use self::iterator::ChangeIterator;
pub use self::composed::Composable;
pub use self::map_pos::MapMode;
pub use self::maped::Mapable;
