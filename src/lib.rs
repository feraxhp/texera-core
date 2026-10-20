// Copyright (C) 2026 feraxhp
// 
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
// 
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
// 
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! # Texera (`texera`)
//!
//! **Texera** is a modular, immutable editor state engine in Rust, faithfully ported from
//! [CodeMirror 6 (`@codemirror/state`)](https://codemirror.net/docs/ref/#state).
//!
//! It follows a functional, transactional architecture where an immutable [`State`] holds the
//! document ([`Text`]), [`Selection`], and active [`Extension`]s. Every modification is described
//! declaratively via a [`TransactionSpec`] (or the [`tr!`] macro) and applied through
//! [`State::update`], producing a [`TransactionResult`] with the new [`State`].
//!
//! ## Core Architecture & Use Cases
//!
//! ### 1. Creating a State and Applying Transactions
//!
//! ```rust
//! use texera_core::{Selection, State, tr};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let state = State::from("hello world");
//! assert_eq!(state.doc().to_string(), "hello world");
//!
//! // Replace "world" with "rust" and move the cursor to the end
//! let result = state.update(tr!({
//!     changes: { from: 6, to: 11, insert: "rust" },
//!     selection: Selection::cursor(10),
//!     user_event: "input.type",
//! }))?;
//!
//! assert!(result.doc_changed());
//! assert!(result.is_user_event("input"));
//! assert_eq!(result.state_ref().doc().to_string(), "hello rust");
//! assert_eq!(result.state_ref().selection().main().head(), 10);
//! # Ok(())
//! # }
//! ```
//!
//! ### 2. Multi-Cursor Editing (`AllowMultipleSelections`)
//!
//! ```rust
//! use texera_core::{AllowMultipleSelections, Selection, State};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let state = State::with_selection(
//!     "foo bar baz",
//!     Selection::from([
//!         Selection::cursor(0),
//!         Selection::cursor(4),
//!         Selection::cursor(8),
//!     ]),
//! )
//! .with_extensions(AllowMultipleSelections(true));
//!
//! let spec = state.replace_selection(">>");
//! let next = state.update(spec)?.into_state();
//! assert_eq!(next.doc().to_string(), ">>foo >>bar >>baz");
//! # Ok(())
//! # }
//! ```
//!
//! ### 3. Facets, Custom StateFields, and Dynamic Reconfiguration (`Compartment`)
//!
//! ```rust
//! use texera_core::{
//!     Compartment, Extension, Facet, IntoExtension, Multi, State, StateField, TabSize,
//!     Transaction, dep, tr,
//! };
//!
//! // Define a multi-value facet
//! #[derive(Clone, Debug, PartialEq, Eq)]
//! struct Tag(&'static str);
//! impl Facet for Tag {
//!     type Mode = Multi;
//!     fn compare_input(a: &Self, b: &Self) -> bool { a == b }
//! }
//!
//! // Define a custom StateField that counts document edits
//! struct EditCount;
//! impl StateField for EditCount {
//!     type Value = usize;
//!     fn create(_: &State) -> usize { 0 }
//!     fn update(val: &usize, tr: &Transaction) -> usize {
//!         if tr.doc_changed() { val + 1 } else { *val }
//!     }
//! }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let tab_comp = Compartment::new();
//! let state = State::from("fn main() {}").with_extensions(Extension::from([
//!     EditCount::init(),
//!     tab_comp.of(TabSize(2)),
//!     Tag("rust").into_ext(),
//! ]));
//!
//! assert_eq!(state.tab_size(), 2);
//! assert_eq!(state.field::<EditCount>(), Some(&0));
//!
//! // Dynamically reconfigure the compartment and edit the document in one transaction
//! let state = state.update(tr!({
//!     changes: { from: 0, insert: "// header\n" },
//!     effects: [tab_comp.reconfigure(TabSize(4))],
//! }))?.into_state();
//!
//! assert_eq!(state.tab_size(), 4);
//! assert_eq!(state.field::<EditCount>(), Some(&1));
//! # Ok(())
//! # }
//! ```
//!
//! ### 4. Transaction Filtering, Change Filtering, and Extenders
//!
//! ```rust
//! use texera_core::{Annotation, AnnotationValue, ChangeFilter, State, TransactionExtender, tr};
//!
//! #[derive(Clone, Debug, PartialEq, Eq)]
//! struct Timestamp(u64);
//! impl AnnotationValue for Timestamp {}
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Protect the first 4 characters ("LOCK") from being modified
//! let state = State::from("LOCK: editable").with_extensions(Extension::from([
//!     ChangeFilter::new(|_| [0, 4]),
//!     TransactionExtender::new(|_| tr!({ annotations: [Annotation::new(Timestamp(1234))] })),
//! ]));
//! # use texera_core::Extension;
//!
//! let res = state.update(tr!({ changes: { from: 0, to: 14, insert: "NEW" } }))?;
//! assert_eq!(res.state_ref().doc().to_string(), "LOCKNEW");
//! assert_eq!(res.annotation::<Timestamp>(), Some(&Timestamp(1234)));
//! # Ok(())
//! # }
//! ```
//!
//! ### 5. RangeSets for Decorations, Markers, and Code Folding
//!
//! ```rust
//! use texera_core::{ChangeSet, ChangeSpec, MapMode, RangeSet, RangeValue};
//!
//! #[derive(Clone, Debug, PartialEq, Eq)]
//! struct Highlight(&'static str);
//! impl RangeValue for Highlight {
//!     fn eq(&self, other: &Self) -> bool { self == other }
//!     fn map_mode(&self) -> MapMode { MapMode::TrackDel }
//! }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let set = RangeSet::of([Highlight("keyword").range(0, 2)], true);
//! let changes = ChangeSet::from_slice(&[ChangeSpec::new(0, Some(0), Some("   ".into()))?], 10)?;
//! let mapped = set.map(changes.description());
//!
//! let ranges: Vec<_> = mapped.iter().map(|r| (r.from, r.to, r.value)).collect();
//! assert_eq!(ranges, vec![(3, 5, Highlight("keyword"))]);
//! # Ok(())
//! # }
//! ```

mod transaction;
mod extension;
mod selection;
mod rangeset;
mod category;
mod changes;
mod errors;
mod state;
mod text;

/// Type-erased thread-safe dynamic value (`dyn Any + Send + Sync`) used internally by
/// facets, state fields, annotations, and state effects.
pub type AnySync = dyn std::any::Any + Send + Sync;

pub use errors::ErrorTypes;

pub use category::find_cluster_break;
pub use category::make_categorizer;
pub use category::CharCategorizer;
pub use category::CategorizeInput;
pub use category::CharCategory;
pub use category::count_column;
pub use category::find_column;
pub use category::WordChars;

pub use rangeset::RangeSetBuilder;
pub use rangeset::RangeComparator;
pub use rangeset::RangeSetCursor;
pub use rangeset::RangeSetUpdate;
pub use rangeset::SpanIterator;
pub use rangeset::RangeValue;
pub use rangeset::RangeSet;
pub use rangeset::Chunk;
pub use rangeset::Range;

pub use changes::ChangeIterator;
pub use changes::SimpleChange;
pub use changes::Composable;
pub use changes::ChangeSpec;
pub use changes::ChangeDesc;
pub use changes::ChangeSet;
pub use changes::ChangeMap;
pub use changes::Position;
pub use changes::MapMode;
pub use changes::Mapable;
pub use changes::Touched;
pub use changes::Change;

pub use selection::SelectionIntoIter;
pub use selection::SelectionRange;
pub use selection::SelectionSpec;
pub use selection::Selection;
pub use selection::Assoc;

pub use transaction::AnnotationValue;
pub use transaction::TransactionSpec;
pub use transaction::Transaction;
pub use transaction::StateEffect;
pub use transaction::Annotation;
pub use transaction::UserEvent;
pub use transaction::Effect;

pub use text::Line;
pub use text::Text;

pub use state::AllowMultipleSelections;
pub use state::IntoTransactionSpecs;
pub use state::TransactionExtender;
pub use state::ChangeFilterResult;
pub use state::TransactionFilter;
pub use state::TransactionResult;
pub use state::LineSeparator;
pub use state::ChangeFilter;
pub use state::ReadOnly;
pub use state::TabSize;
pub use state::State;

pub use extension::ErasedFacetProvider;
pub use extension::ErasedStateField;
pub use extension::IntoExtension;
pub use extension::Compartment;
pub use extension::StateField;
pub use extension::FacetMode;
pub use extension::Extension;
pub use extension::FacetDef;
pub use extension::Priority;
pub use extension::Single;
pub use extension::Multi;
pub use extension::Facet;
pub use extension::dep;
