use crate::changes::Composable;
use crate::{Annotation, AnnotationValue, ChangeSet, Selection, State, StateEffect, Text, TransactionSpec, UserEvent};


/// An active, resolved transaction passed to [`TransactionFilter`](crate::TransactionFilter),
/// [`ChangeFilter`](crate::ChangeFilter), [`TransactionExtender`](crate::TransactionExtender),
/// and [`StateField::update`](crate::StateField::update) during [`State::update`](crate::State::update).
///
/// Provides inspection of the starting state ([`Transaction::start_state`]), the resulting document
/// ([`Transaction::doc`]), the resolved [`ChangeSet`] ([`Transaction::changes`]), its undo inverse
/// ([`Transaction::inverted_changes`]), [`Selection`], [`StateEffect`]s, and [`Annotation`]s.
///
/// # Examples
///
/// ```rust
/// use texera_core::{ChangeSet, Composable, State, StateField, Transaction, tr};
///
/// struct UndoHistory;
/// impl StateField for UndoHistory {
///     type Value = Vec<ChangeSet>;
///     fn create(_: &State) -> Self::Value { Vec::new() }
///     fn update(stack: &Self::Value, tx: &Transaction) -> Self::Value {
///         let mut next = stack.clone();
///         if tx.doc_changed() && tx.is_user_event("input") {
///             next.push(tx.inverted_changes());
///         }
///         next
///     }
/// }
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let state = State::from("hello").with_extensions(UndoHistory::init());
/// let state = state.update(tr!({
///     changes: { from: 5, insert: "!" },
///     user_event: "input.type",
/// }))?.into_state();
///
/// let history = state.field::<UndoHistory>().unwrap();
/// assert_eq!(history.len(), 1);
/// assert_eq!(history[0].clone().apply(state.doc().clone())?.to_string(), "hello");
/// # Ok(())
/// # }
/// ```
#[derive(Default, Clone, Debug)]
pub struct Transaction {
    /// Resolved change set if any document edits were specified.
    pub changes: Option<ChangeSet>,
    /// Explicit selection specified on the transaction, if any.
    pub selection: Option<Selection>,
    /// Annotations attached to this transaction.
    pub annotations: Vec<Annotation>,
    /// State effects attached to this transaction.
    pub effects: Vec<StateEffect>,
    /// Whether the transaction requests scrolling the primary cursor into view.
    pub scroll_into_view: bool,
    /// Document resulting from applying `changes` to `start_state.doc()`.
    pub doc: Text,
    /// State prior to applying this transaction.
    pub start_state: State,
    /// Original specification that produced this transaction.
    pub spec: TransactionSpec,
}

impl Transaction {
    /// Returns `true` if this transaction modifies the document (`!self.changes().is_empty()`).
    #[inline]
    pub fn doc_changed(&self) -> bool {
        self.changes.as_ref().is_some_and(|c| !c.is_empty())
    }

    /// Returns `true` if either the document was modified or an explicit selection was set.
    #[inline]
    pub fn selection_changed(&self) -> bool {
        self.doc_changed() || self.selection.is_some()
    }

    /// Returns a reference to the new document after applying this transaction's changes.
    #[inline]
    pub fn doc(&self) -> &Text {
        &self.doc
    }

    /// Returns a reference to the [`State`] before this transaction started.
    #[inline]
    pub fn start_state(&self) -> &State {
        &self.start_state
    }

    /// Returns the [`ChangeSet`] for this transaction (an identity change set if no edits occurred).
    #[inline]
    pub fn changes(&self) -> ChangeSet {
        self.changes.clone().unwrap_or_else(|| {
            ChangeSet::from_slice(&[], self.start_state.doc().len_chars()).unwrap()
        })
    }

    /// Returns the inverted [`ChangeSet`] that undoes this transaction's edits when applied to [`Transaction::doc`].
    #[inline]
    pub fn inverted_changes(&self) -> ChangeSet {
        match &self.changes {
            Some(ch) => ch.invert(self.start_state.doc()),
            None => ChangeSet::from_slice(&[], self.start_state.doc().len_chars()).unwrap(),
        }
    }

    /// Returns the explicit [`Selection`] set on this transaction, if any.
    #[inline]
    pub fn selection(&self) -> Option<&Selection> {
        self.selection.as_ref()
    }

    /// Returns the slice of [`StateEffect`]s attached to this transaction.
    #[inline]
    pub fn effects(&self) -> &[StateEffect] {
        &self.effects
    }

    /// Returns the slice of [`Annotation`]s attached to this transaction.
    #[inline]
    pub fn annotations(&self) -> &[Annotation] {
        &self.annotations
    }

    /// Queries the first [`Annotation`] of type `A` attached to this transaction.
    #[inline]
    pub fn annotation<A: AnnotationValue>(&self) -> Option<&A> {
        self.annotations.iter().find_map(|a| a.get::<A>())
    }

    /// Returns `true` if this transaction carries a [`UserEvent`] annotation matching `event`
    /// (exact match or dot-prefix match, e.g. `"input"` matches `"input.type"`).
    #[inline]
    pub fn is_user_event(&self, event: &str) -> bool {
        self.annotation::<UserEvent>()
            .is_some_and(|ue| ue.matches(event))
    }

    /// Returns `true` if this transaction requests scrolling the cursor into view.
    #[inline]
    pub fn scroll_into_view(&self) -> bool {
        self.scroll_into_view
    }

    /// Returns a reference to the underlying [`TransactionSpec`].
    #[inline]
    pub fn spec(&self) -> &TransactionSpec {
        &self.spec
    }
}

impl TryFrom<(usize, TransactionSpec)> for Transaction {
    type Error = mynox::Error;

    fn try_from((doc_len, value): (usize, TransactionSpec)) -> Result<Self, Self::Error> {
        let spec = value.clone();
        let changes = if value.changes.is_empty() {
            None
        } else {
            Some(ChangeSet::from_slice(value.changes.as_slice(), doc_len)?)
        };
        
        let mut annotations = value.annotations;
        if let Some(user_event) = value.user_event {
            annotations.push(Annotation::user_event(user_event));
        }
        
        Ok(Self {
            changes,
            selection: value.selection,
            annotations,
            effects: value.effects,
            scroll_into_view: value.scroll_into_view.unwrap_or(false),
            doc: Text::default(),
            start_state: State::default(),
            spec,
        })
    }
}
