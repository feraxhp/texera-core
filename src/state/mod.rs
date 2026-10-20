//! Editor state container, built-in facets, and transaction execution pipeline.
//!
//! [`State`] is the central immutable value representing a snapshot of an editor at a
//! point in time: its [`Text`] document, active [`Selection`], compiled [`Configuration`],
//! [`StateField`](crate::StateField) values, and evaluated [`Facet`](crate::Facet) outputs.
//!
//! To advance the editor to a new state, construct one or more [`TransactionSpec`](crate::TransactionSpec)s
//! (or use the [`tr!`](crate::tr) macro) and pass them to [`State::update`], which returns a
//! [`TransactionResult`] containing the new [`State`] along with full metadata about the transition.

pub mod configuration;
pub mod extensions;
pub mod update;
pub mod facet;
pub mod init;
mod test;

use std::collections::HashMap;
use std::sync::Arc;

use crate::changes::Composable;
use crate::{Annotation, AnnotationValue, AnySync, ChangeSet, Selection, StateEffect, Text, UserEvent};

pub use self::configuration::Configuration;
pub use self::facet::{
    AllowMultipleSelections, ChangeFilter, ChangeFilterResult, LineSeparator, ReadOnly, TabSize,
    TransactionExtender, TransactionFilter,
};
pub use self::update::IntoTransactionSpecs;

/// An immutable snapshot of an editor's document, selection, configuration, and extension state.
///
/// Cloning a `State` is cheap ($O(1)$ for the document rope, configuration, and `Arc`-backed
/// extension slots). State transitions are performed deterministically via [`State::update`].
///
/// # Examples
///
/// ```
/// use texera_core::{State, Selection, AllowMultipleSelections, Extension, TabSize, tr};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let state = State::from("fn main() {}")
///     .with_extensions(Extension::from([
///         AllowMultipleSelections(true).into(),
///         TabSize(2).into(),
///     ]));
///
/// assert_eq!(state.doc().to_string(), "fn main() {}");
/// assert_eq!(state.tab_size(), 2);
///
/// let res = state.update(tr!({
///     changes: { from: 11, to: 11, insert: "\n  println!(\"hi\");\n" },
///     selection: Selection::cursor(29),
///     user_event: "input.type",
/// }))?;
///
/// assert!(res.doc_changed());
/// assert!(res.is_user_event("input"));
/// assert_eq!(res.state_ref().doc().len_lines(), 3);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default, Clone)]
pub struct State {
    doc: Text,
    selection: Selection,
    pub(crate) raw_selection: Option<Selection>,
    pub(crate) config: Arc<Configuration>,
    dynamic_slots: Vec<Arc<AnySync>>,
    pub(crate) provider_cache: HashMap<usize, Arc<AnySync>>,
}

impl State {
    /// Returns a reference to the current document [`Text`].
    ///
    /// # Examples
    ///
    /// ```
    /// use texera_core::State;
    ///
    /// let state = State::from("hello\nworld");
    /// assert_eq!(state.doc().len_lines(), 2);
    /// assert_eq!(state.doc().len_chars(), 11);
    /// ```
    pub fn doc<'a>(&'a self) -> &'a Text { &self.doc }

    /// Returns a reference to the current editor [`Selection`].
    ///
    /// # Examples
    ///
    /// ```
    /// use texera_core::{State, Selection};
    ///
    /// let state = State::with_selection("hello", Selection::range(1, 4));
    /// assert_eq!(state.selection().main().from(), 1);
    /// assert_eq!(state.selection().main().to(), 4);
    /// ```
    pub fn selection<'a>(&'a self) -> &'a Selection { &self.selection }
}

/// The resolved outcome of applying a transaction via [`State::update`].
///
/// Contains the resulting [`State`], the starting [`State`], the applied [`ChangeSet`]s,
/// updated [`Selection`]s, [`Annotation`]s, [`StateEffect`]s, and helper methods for
/// inspecting or inverting the changes (e.g. for undo history).
///
/// # Examples
///
/// ```
/// use texera_core::{Composable, State, tr};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let state = State::from("hello");
/// let res = state.update(tr!({
///     changes: { from: 5, insert: " world" },
///     user_event: "input.paste",
///     scroll_into_view: true,
/// }))?;
///
/// assert!(res.doc_changed());
/// assert!(res.selection_changed());
/// assert!(res.scroll_into_view());
/// assert!(res.is_user_event("input"));
/// assert_eq!(res.state_ref().doc().to_string(), "hello world");
///
/// // Invert changes to undo the edit:
/// let undo = res.inverted_changes();
/// let restored = undo.apply(res.state_ref().doc().clone())?;
/// assert_eq!(restored.to_string(), "hello");
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct TransactionResult {
    pub(crate) state: State,
    pub(crate) start_state: State,
    pub(crate) changes: Vec<ChangeSet>,
    pub(crate) selection: Vec<Selection>,
    pub(crate) annotations: Vec<Annotation>,
    pub(crate) effects: Vec<StateEffect>,
    pub(crate) scroll_into_view: bool,
}

impl TransactionResult {
    /// Returns a clone of the new [`State`] produced by the transaction.
    ///
    /// Prefer [`TransactionResult::state_ref`] to borrow without cloning, or
    /// [`TransactionResult::into_state`] to consume `self`.
    pub fn state(&self) -> State { self.state.to_owned() }

    /// Borrows the new [`State`] produced by the transaction.
    pub fn state_ref(&self) -> &State { &self.state }

    /// Consumes `self` and returns the new [`State`] without cloning.
    pub fn into_state(self) -> State { self.state }

    /// Returns a clone of the [`State`] before the transaction was applied.
    pub fn start_state(&self) -> State { self.start_state.to_owned() }

    /// Borrows the [`State`] before the transaction was applied.
    pub fn start_state_ref(&self) -> &State { &self.start_state }

    /// Returns a cloned vector of the [`ChangeSet`]s applied during this transaction.
    pub fn changes(&self) -> Vec<ChangeSet> { self.changes.to_owned() }

    /// Borrows the slice of [`ChangeSet`]s applied during this transaction.
    pub fn changes_ref(&self) -> &[ChangeSet] { &self.changes }

    /// Returns the explicit [`Selection`]s recorded during this transaction.
    pub fn selection(&self) -> Vec<Selection> { self.selection.to_owned() }

    /// Returns the slice of [`StateEffect`]s emitted during this transaction.
    pub fn effects(&self) -> &[StateEffect] { &self.effects }

    /// Returns the slice of [`Annotation`]s attached to this transaction.
    pub fn annotations(&self) -> &[Annotation] { &self.annotations }

    /// Returns `true` if this transaction requested that the primary cursor be scrolled into view.
    pub fn scroll_into_view(&self) -> bool { self.scroll_into_view }

    /// Returns `true` if any non-empty document changes were applied.
    pub fn doc_changed(&self) -> bool {
        self.changes.iter().any(|c| !c.is_empty())
    }

    /// Returns `true` if the document changed or a new selection was explicitly set.
    pub fn selection_changed(&self) -> bool {
        self.doc_changed() || !self.selection.is_empty()
    }

    /// Computes a [`ChangeSet`] that inverses the changes made by this transaction, suitable
    /// for undo history.
    ///
    /// If the transaction made no changes, returns an empty identity [`ChangeSet`] for the document.
    pub fn inverted_changes(&self) -> ChangeSet {
        match self.changes.first() {
            Some(ch) => ch.invert(self.start_state.doc()),
            None => ChangeSet::from_slice(&[], self.start_state.doc().len_chars()).unwrap(),
        }
    }

    /// Queries the first annotation of type `A` attached to this transaction.
    ///
    /// # Examples
    ///
    /// ```
    /// use texera_core::{State, Annotation, AnnotationValue, tr};
    ///
    /// #[derive(Clone, Debug, PartialEq)]
    /// struct Source(&'static str);
    /// impl AnnotationValue for Source {}
    ///
    /// let state = State::from("abc");
    /// let res = state.update(tr!({
    ///     annotations: [Annotation::new(Source("lsp"))],
    /// })).unwrap();
    ///
    /// assert_eq!(res.annotation::<Source>(), Some(&Source("lsp")));
    /// ```
    pub fn annotation<A: AnnotationValue>(&self) -> Option<&A> {
        self.annotations.iter().find_map(|a| a.get::<A>())
    }

    /// Checks whether this transaction carries a [`UserEvent`] matching `event` (exact or
    /// dot-separated prefix match, e.g. `"input"` matches `"input.type"`).
    pub fn is_user_event(&self, event: &str) -> bool {
        self.annotation::<UserEvent>()
            .is_some_and(|ue| ue.matches(event))
    }
}
