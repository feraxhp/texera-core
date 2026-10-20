use std::collections::HashMap;
use std::sync::Arc;

use mynox::Error;

use crate::{ChangeSpec, Extension, Selection, SelectionRange, Text, TransactionSpec};

use super::State;


impl From<&str> for State {
    fn from(value: &str) -> Self {
        Self { 
            doc: Text::from(value), 
            selection: Selection::new(),
            raw_selection: None,
            config: Arc::default(),
            dynamic_slots: Vec::new(),
            provider_cache: HashMap::new(),
        }
    }
}

impl From<Extension> for State {
    fn from(value: Extension) -> Self {
        Self::default().with_extensions(value)
    }
}

impl State {
    /// Creates a new [`State`] initialized with `doc` and `selection`.
    ///
    /// Note: If `selection` contains multiple ranges, it is initially coerced to a single
    /// range unless the [`AllowMultipleSelections`](crate::AllowMultipleSelections) facet
    /// is enabled via [`State::with_extensions`]. When [`AllowMultipleSelections(true)`](crate::AllowMultipleSelections)
    /// is attached, the original multi-range selection passed here is automatically restored.
    ///
    /// # Examples
    ///
    /// ```
    /// use texera_core::{State, Selection, AllowMultipleSelections};
    ///
    /// let sel = Selection::from_with_head([Selection::range(0, 2), Selection::range(4, 6)], 1).unwrap();
    /// let state = State::with_selection("one two", sel)
    ///     .with_extensions(AllowMultipleSelections(true));
    ///
    /// assert_eq!(state.selection().count(), 2);
    /// assert_eq!(state.selection().main().from(), 4);
    /// ```
    pub fn with_selection<V>(doc: &str, selection: V) -> State 
    where 
        V: Into<Selection>
    {
        let raw: Selection = selection.into();
        let single = raw.clone().as_single();
        Self { 
            doc: Text::from(doc), 
            selection: single,
            raw_selection: Some(raw),
            ..Default::default()
        }
    }
    
    /// Returns an iterator over all `(SelectionRange, is_main)` pairs in the current selection.
    ///
    /// # Examples
    ///
    /// ```
    /// use texera_core::{State, Selection, AllowMultipleSelections};
    ///
    /// let state = State::with_selection("abc def", Selection::from([Selection::range(0, 3), Selection::range(4, 7)]))
    ///     .with_extensions(AllowMultipleSelections(true));
    ///
    /// let ranges: Vec<_> = state.iter_selections().collect();
    /// assert_eq!(ranges.len(), 2);
    /// assert!(ranges[1].1); // last range is primary
    /// ```
    pub fn iter_selections(&self) -> impl Iterator<Item = (SelectionRange, bool)> {
        self.selection.clone().into_iter()
    }

    /// Creates a [`TransactionSpec`] that replaces every range in the current selection with `text`.
    ///
    /// Respects any configured [`LineSeparator`](crate::LineSeparator) when normalizing `text`.
    ///
    /// # Examples
    ///
    /// ```
    /// use texera_core::{State, Selection};
    ///
    /// let state = State::with_selection("hello world", Selection::range(6, 11));
    /// let spec = state.replace_selection("rust");
    /// let res = state.update(spec).unwrap();
    ///
    /// assert_eq!(res.state_ref().doc().to_string(), "hello rust");
    /// ```
    pub fn replace_selection(&self, text: &str) -> TransactionSpec {
        let text = self.to_text(text);
        let changes: Vec<_> = self
            .selection
            .iter()
            .map(|(s, _)| {
                let from = s.from().min(s.to());
                ChangeSpec::new(from, Some(s.to()), Some(text.clone()))
                    .expect("Selection range invariant guarantees from <= to")
            })
            .collect();

        TransactionSpec::default().with_changes(changes)
    }

    /// Applies `map` to each `(SelectionRange, is_main)` in the current selection and combines
    /// all resulting [`TransactionSpec`]s into a single merged [`TransactionSpec`], mapping
    /// selection positions across changes automatically.
    ///
    /// This is the idiomatic helper for multi-cursor commands (e.g. wrapping each cursor/selection
    /// in quotes or brackets, indenting, or moving cursors).
    ///
    /// # Examples
    ///
    /// ```
    /// use texera_core::{State, Selection, AllowMultipleSelections, tr};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let state = State::with_selection("foo bar", Selection::from([Selection::range(0, 3), Selection::range(4, 7)]))
    ///     .with_extensions(AllowMultipleSelections(true));
    ///
    /// // Wrap each selected word in parentheses and keep it selected:
    /// let spec = state.change_by_range(|(range, _is_main)| {
    ///     let word = state.slice_doc(Some(range.from()), Some(range.to()));
    ///     let wrapped = format!("({word})");
    ///     let len = wrapped.chars().count();
    ///     Ok(tr!({
    ///         changes: { from: range.from(), to: range.to(), insert: wrapped.as_str() },
    ///         selection: Selection::range(range.from(), range.from() + len),
    ///     }))
    /// })?;
    ///
    /// let res = state.update(spec)?;
    /// assert_eq!(res.state_ref().doc().to_string(), "(foo) (bar)");
    /// # Ok(())
    /// # }
    /// ```
    pub fn change_by_range<F>(&self, map: F) -> Result<TransactionSpec, Error>
    where
        F: Fn((SelectionRange, bool)) -> Result<TransactionSpec, Error>,
    {
        let trs = self.iter_selections().map(map);
        TransactionSpec::try_from_iter_result(trs)
    }
}
