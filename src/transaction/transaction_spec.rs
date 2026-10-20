use mynox::Error;

use crate::ChangeSpec;
use crate::Selection;

use super::Annotation;
use super::StateEffect;


/// Builder specification describing a transaction to be applied via [`State::update`](crate::State::update).
///
/// A `TransactionSpec` can be constructed either with builder methods (`TransactionSpec::default().with_changes(...)`)
/// or declaratively via the [`tr!`](crate::tr) macro.
///
/// # Examples
///
/// ```rust
/// use texera_core::{ChangeSpec, Selection, State, TransactionSpec};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let spec = TransactionSpec::default()
///     .with_change(ChangeSpec::new(0, Some(5), Some("Hi".into()))?)
///     .with_selection(Selection::cursor(2))
///     .with_user_event("input.type")
///     .is_scroll_into_view(true);
///
/// let res = State::from("hello world").update(spec)?;
/// assert_eq!(res.state_ref().doc().to_string(), "Hi world");
/// assert_eq!(res.state_ref().selection().main().head(), 2);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Default)]
pub struct TransactionSpec {
    pub(crate) changes: Vec<ChangeSpec>,
    pub(crate) selection: Option<Selection>,
    pub(crate) effects: Vec<StateEffect>,
    pub(crate) annotations: Vec<Annotation>,
    pub(crate) user_event: Option<String>,
    pub(crate) scroll_into_view: Option<bool>,
    pub(crate) filter: Option<bool>,
    pub(crate) sequential: Option<bool>,
}

impl TransactionSpec {
    /// Appends a single [`ChangeSpec`] to this transaction specification.
    pub fn with_change(mut self, change: ChangeSpec) -> Self {
        self.changes.push(change);
        self
    }

    /// Appends multiple [`ChangeSpec`]s to this transaction specification.
    pub fn with_changes(mut self, changes: impl IntoIterator<Item = ChangeSpec>) -> Self {
        self.changes.extend(changes);
        self
    }

    /// Sets the explicit new [`Selection`] (or [`SelectionRange`](crate::SelectionRange)) for this transaction.
    pub fn with_selection(mut self, selection: impl Into<Selection>) -> Self {
        self.selection = Some(selection.into());
        self
    }

    /// Attaches a single [`StateEffect`] to this transaction specification.
    pub fn with_effect(mut self, effect: StateEffect) -> Self {
        self.effects.push(effect);
        self
    }

    /// Attaches multiple [`StateEffect`]s to this transaction specification.
    pub fn with_effects(mut self, effects: impl IntoIterator<Item = StateEffect>) -> Self {
        self.effects.extend(effects);
        self
    }

    /// Attaches a single [`Annotation`] to this transaction specification.
    pub fn with_annotation(mut self, annotation: Annotation) -> Self {
        self.annotations.push(annotation);
        self
    }

    /// Attaches multiple [`Annotation`]s to this transaction specification.
    pub fn with_annotations(mut self, annotations: impl IntoIterator<Item = Annotation>) -> Self {
        self.annotations.extend(annotations);
        self
    }

    /// Attaches a [`UserEvent`](crate::UserEvent) annotation (e.g. `"input.type"`, `"delete.selection"`).
    pub fn with_user_event(mut self, user_event: impl Into<String>) -> Self {
        self.user_event = Some(user_event.into());
        self
    }

    /// Sets whether the view should scroll the main selection head into view after this transaction.
    pub fn is_scroll_into_view(mut self, value: bool) -> Self {
        self.scroll_into_view = Some(value);
        self
    }

    /// Controls whether [`ChangeFilter`](crate::ChangeFilter) and [`TransactionFilter`](crate::TransactionFilter)
    /// facets are run for this transaction (defaults to `true`).
    pub fn is_filter(mut self, value: bool) -> Self {
        self.filter = Some(value);
        self
    }

    /// When combining multiple `TransactionSpec`s, marks this spec's changes and selection as
    /// expressed in the coordinate system produced by the preceding specs (`sequential: true`),
    /// rather than in the original `start_state` document coordinate system.
    pub fn is_sequential(mut self, value: bool) -> Self {
        self.sequential = Some(value);
        self
    }

    /// Combines an iterator of range-specific `TransactionSpec`s (such as those produced by
    /// [`State::change_by_range`](crate::State::change_by_range)), shifting each subsequent spec's
    /// selection by the cumulative length delta of prior changes.
    pub fn try_from_iter(iter: impl IntoIterator<Item = TransactionSpec>) -> Result<Self, Error> {
        let mut offset: isize = 0;
        let mut ranges = Vec::new();
        let mut changes = Vec::new();
        let mut effects = Vec::new();
        let mut annotations = Vec::new();

        for tr in iter {
            if let Some(selection) = tr.selection {
                for r in selection.ranges() {
                    let anchor = (r.anchor() as isize + offset) as usize;
                    let head = (r.head() as isize + offset) as usize;
                    ranges.push(Selection::range(anchor, head));
                }
            }

            let diff: isize = tr.changes.iter().map(|c| {
                let insert_len = c.insert.as_ref().map(|t| t.len_chars()).unwrap_or(0) as isize;
                let delete_len = (c.to - c.from) as isize;
                insert_len - delete_len
            }).sum();

            offset += diff;
            changes.extend(tr.changes);
            effects.extend(tr.effects);
            annotations.extend(tr.annotations);
        }

        let mut spec = TransactionSpec::default()
            .with_changes(changes)
            .with_effects(effects)
            .with_annotations(annotations);

        if !ranges.is_empty() {
            let selection: Selection = ranges.into_iter().collect();
            spec = spec.with_selection(selection);
        }

        Ok(spec)
    }

    /// Fallible variant of [`TransactionSpec::try_from_iter`] that short-circuits on the first `Err`.
    pub fn try_from_iter_result(iter: impl IntoIterator<Item = Result<TransactionSpec, Error>>) -> Result<Self, Error> {
        let mut collected = Vec::new();
        for item in iter {
            collected.push(item?);
        }
        Self::try_from_iter(collected)
    }
}


/// Declarative macro for constructing a [`TransactionSpec`] with CodeMirror 6-style object literal syntax.
///
/// # Supported Fields
///
/// - `changes: { from: .., to: .., insert: .. }` or `changes: [ { .. }, { .. } ]` (delegates to [`ch!`](crate::ch))
/// - `selection: <Selection or SelectionRange>`
/// - `effects: [ effect1, effect2 ]`
/// - `annotations: [ ann1, ann2 ]`
/// - `user_event: "input.type"`
/// - `scroll_into_view: true`
/// - `filter: false`
/// - `sequential: true`
///
/// Note: when `changes` is specified, [`ch!`](crate::ch) validates the range bounds with `?`, so the
/// enclosing function must return a `Result`.
///
/// # Examples
///
/// ```rust
/// use texera_core::{Selection, State, tr};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let empty_spec = tr!();
/// let edit_spec = tr!({
///     changes: [
///         { from: 0, to: 5, insert: "Greetings" },
///         { from: 5, insert: "!" },
///     ],
///     selection: Selection::cursor(10),
///     user_event: "input.type",
///     scroll_into_view: true,
/// });
///
/// let state = State::from("hello").update(empty_spec)?.into_state();
/// let res = state.update(edit_spec)?;
/// assert_eq!(res.state_ref().doc().to_string(), "Greetings!");
/// assert!(res.is_user_event("input"));
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! tr {
    // Base case: no more fields left to parse
    (@munch $acc:expr $(,)?) => {$acc
    };

    // Multiple changes: changes: [ { ... }, { ... } ]
    (@munch $acc:expr, changes: [$($changes:tt)* ]$(, $($rest:tt)*)?) => {
        $crate::tr!(@munch $acc.with_changes($crate::ch!([$($changes)* ])),$($($rest)*)?)
    };

    // Single change: changes: { ... }
    (@munch $acc:expr, changes: {$($change:tt)* }$(, $($rest:tt)*)?) => {
        $crate::tr!(@munch $acc.with_change($crate::ch!({$($change)* })),$($($rest)*)?)
    };

    // Selection
    (@munch $acc:expr, selection: $selection:expr$(, $($rest:tt)*)?) => {
        $crate::tr!(@munch $acc.with_selection($selection),$($($rest)*)?)
    };

    // Effects list: effects: [ ef1, ef2 ]
    (@munch $acc:expr, effects: [$($effect:expr),*$(,)? ] $(,$($rest:tt)*)?) => {$crate::tr!(@munch $acc.with_effects([$($effect),*]),$($($rest)*)?)
    };

    // Annotations list: annotations: [ an1, an2 ]
    (@munch $acc:expr, annotations: [$($annotation:expr),*$(,)? ] $(,$($rest:tt)*)?) => {$crate::tr!(@munch $acc.with_annotations([$($annotation),*]),$($($rest)*)?)
    };

    // User event
    (@munch $acc:expr, user_event: $event:expr$(, $($rest:tt)*)?) => {
        $crate::tr!(@munch $acc.with_user_event($event),$($($rest)*)?)
    };

    // Scroll into view
    (@munch $acc:expr, scroll_into_view: $val:expr$(, $($rest:tt)*)?) => {
        $crate::tr!(@munch $acc.is_scroll_into_view($val),$($($rest)*)?)
    };

    // Filter
    (@munch $acc:expr, filter: $val:expr$(, $($rest:tt)*)?) => {
        $crate::tr!(@munch $acc.is_filter($val),$($($rest)*)?)
    };

    // Sequential
    (@munch $acc:expr, sequential: $val:expr$(, $($rest:tt)*)?) => {
        $crate::tr!(@munch $acc.is_sequential($val),$($($rest)*)?)
    };

    // Catch unknown fields or invalid syntax early
    (@munch $acc:expr, $($invalid:tt)+) => {
        compile_error!(concat!("Invalid field or syntax in `tr!`: ", stringify!($($invalid)+)))
    };

    // Entry point with braces: tr!({ ... })
    ({ $($body:tt)* }$(,)?) => {
        $crate::tr!(@munch $crate::TransactionSpec::default(), $($body)*)
    };

    // Empty entry point: tr!()
    () => {
        $crate::TransactionSpec::default()
    };
}
