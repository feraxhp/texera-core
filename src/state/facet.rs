use std::{any::TypeId, sync::Arc};

use mynox::Error;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    AnySync, CharCategory, ErasedFacetProvider, Extension, Selection, SelectionRange, Text,
    Transaction, TransactionSpec,
    category::{CharCategorizer, WordChars},
    extension::{Facet, FacetDef, Multi, StateField},
};

use super::{State, configuration::SlotAddr};


/// Built-in facet that controls whether the editor allows selections with multiple ranges (multi-cursor).
///
/// Defaults to `false`. When multiple extensions provide [`AllowMultipleSelections`],
/// the combined output is `true` if **any** input is `true` (logical OR).
///
/// When `false`, any multi-range [`Selection`] is automatically collapsed to its primary range.
///
/// # Examples
///
/// ```
/// use texera_core::{State, Selection, AllowMultipleSelections};
///
/// let sel = Selection::from([Selection::range(0, 0), Selection::range(5, 5)]);
/// let single_state = State::with_selection("hello world", sel.clone());
/// assert_eq!(single_state.selection().count(), 1);
///
/// let multi_state = State::with_selection("hello world", sel)
///     .with_extensions(AllowMultipleSelections(true));
/// assert_eq!(multi_state.selection().count(), 2);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AllowMultipleSelections(pub bool);

impl FacetDef for AllowMultipleSelections {
    type Input = AllowMultipleSelections;
    type Output = bool;

    fn combine(inputs: &[Self::Input]) -> bool {
        inputs.iter().any(|x| x.0)
    }

    fn compare_input(a: &Self::Input, b: &Self::Input) -> bool {
        a == b
    }

    fn compare(a: &bool, b: &bool) -> bool {
        a == b
    }
}

impl From<AllowMultipleSelections> for Extension {
    fn from(value: AllowMultipleSelections) -> Self {
        Extension::facet::<AllowMultipleSelections>(value)
    }
}

/// Built-in facet indicating whether the editor content should be treated as read-only by UI commands.
///
/// Defaults to `false`. Combines via logical OR (`true` if any registered input is `true`).
/// Query via [`State::read_only`].
///
/// # Examples
///
/// ```
/// use texera_core::{State, ReadOnly};
///
/// let state = State::from("locked").with_extensions(ReadOnly(true));
/// assert!(state.read_only());
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReadOnly(pub bool);

impl FacetDef for ReadOnly {
    type Input = ReadOnly;
    type Output = bool;

    fn combine(inputs: &[Self::Input]) -> bool {
        inputs.iter().any(|x| x.0)
    }

    fn compare_input(a: &Self::Input, b: &Self::Input) -> bool {
        a == b
    }

    fn compare(a: &bool, b: &bool) -> bool {
        a == b
    }
}

impl From<ReadOnly> for Extension {
    fn from(value: ReadOnly) -> Self {
        Extension::facet::<ReadOnly>(value)
    }
}

/// Built-in facet configuring the visual width of a tab character (`'\t'`) in spaces.
///
/// Defaults to `4` when no [`TabSize`] extension is registered. When multiple inputs are
/// provided, the highest-priority (first) input wins. Query via [`State::tab_size`].
///
/// # Examples
///
/// ```
/// use texera_core::{State, TabSize, Extension, Priority};
///
/// let state = State::from("\tcode")
///     .with_extensions(Extension::from([
///         TabSize(2).into(),
///         Extension::from(TabSize(8)).with_priority(Priority::High),
///     ]));
/// assert_eq!(state.tab_size(), 8);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TabSize(pub usize);

impl FacetDef for TabSize {
    type Input = TabSize;
    type Output = usize;

    fn combine(inputs: &[Self::Input]) -> usize {
        inputs.first().map(|x| x.0).unwrap_or(4)
    }

    fn compare_input(a: &Self::Input, b: &Self::Input) -> bool {
        a == b
    }

    fn compare(a: &usize, b: &usize) -> bool {
        a == b
    }
}

impl From<TabSize> for Extension {
    fn from(value: TabSize) -> Self {
        Extension::facet::<TabSize>(value)
    }
}

/// Built-in facet configuring the preferred line separator string (such as `"\n"` or `"\r\n"`).
///
/// Defaults to `None` (which uses `"\n"` for [`State::line_break`]). Query via [`State::line_break`]
/// or use [`State::slice_doc`] and [`State::to_text`].
///
/// # Examples
///
/// ```
/// use texera_core::{State, LineSeparator};
///
/// let state = State::from("a\nb").with_extensions(LineSeparator("\r\n"));
/// assert_eq!(state.line_break(), "\r\n");
/// assert_eq!(state.slice_doc(None, None), "a\r\nb");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineSeparator(pub &'static str);

impl FacetDef for LineSeparator {
    type Input = LineSeparator;
    type Output = Option<&'static str>;

    fn combine(inputs: &[Self::Input]) -> Option<&'static str> {
        inputs.first().map(|x| x.0)
    }

    fn compare_input(a: &Self::Input, b: &Self::Input) -> bool {
        a == b
    }

    fn compare(a: &Option<&'static str>, b: &Option<&'static str>) -> bool {
        a == b
    }
}

impl From<LineSeparator> for Extension {
    fn from(value: LineSeparator) -> Self {
        Extension::facet::<LineSeparator>(value)
    }
}

/// Return value of a [`ChangeFilter`] callback, determining whether a transaction's document
/// changes are allowed, cancelled, or partially restricted.
///
/// Can be constructed directly or converted via `From<bool>`, `From<[usize; N]>`, or `From<Vec<usize>>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChangeFilterResult {
    /// `true` allows all changes; `false` cancels all document changes in the transaction.
    Bool(bool),
    /// Flat list of `[from_1, to_1, from_2, to_2, ...]` character ranges in the start document
    /// that are **protected** (excluded from modification). Any change overlapping a protected
    /// range is split or discarded so the protected region remains untouched.
    Ranges(Vec<usize>),
}

impl From<bool> for ChangeFilterResult {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl<const N: usize> From<[usize; N]> for ChangeFilterResult {
    fn from(value: [usize; N]) -> Self {
        Self::Ranges(value.to_vec())
    }
}

impl From<Vec<usize>> for ChangeFilterResult {
    fn from(value: Vec<usize>) -> Self {
        Self::Ranges(value)
    }
}

/// Built-in facet that inspects a transaction and can cancel or partially mask its document changes.
///
/// Change filters run during [`State::update`] unless the [`TransactionSpec`] sets `filter: false`.
///
/// # Examples
///
/// ```
/// use texera_core::{State, ChangeFilter, tr};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Protect the first 5 characters ("const") from being deleted or overwritten:
/// let state = State::from("const x = 1;")
///     .with_extensions(ChangeFilter::new(|_tr| [0, 5]));
///
/// // Attempt to delete the entire document:
/// let res = state.update(tr!({ changes: { from: 0, to: 12, insert: "" } }))?;
/// // Only characters after index 5 are deleted:
/// assert_eq!(res.state_ref().doc().to_string(), "const");
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct ChangeFilter(pub Arc<dyn Fn(&Transaction) -> ChangeFilterResult + Send + Sync>);

impl ChangeFilter {
    /// Registers a change filter function as an [`Extension`].
    ///
    /// The closure receives the candidate [`Transaction`] and returns anything convertible into
    /// [`ChangeFilterResult`] (`bool`, `[usize; N]`, or `Vec<usize>`).
    pub fn new<F, R>(f: F) -> Extension
    where
        F: Fn(&Transaction) -> R + Send + Sync + 'static,
        R: Into<ChangeFilterResult>,
    {
        Extension::from(Self(Arc::new(move |tr| f(tr).into())))
    }
}

impl Facet for ChangeFilter {
    type Mode = Multi;
}

/// Helper trait allowing [`TransactionFilter::new`] closures to return either
/// `Vec<TransactionSpec>` or `Result<Vec<TransactionSpec>, Error>`.
pub trait IntoFilterSpecs {
    /// Converts the filter callback return value into `Result<Vec<TransactionSpec>, Error>`.
    fn into_specs(self) -> Result<Vec<TransactionSpec>, Error>;
}

impl IntoFilterSpecs for Vec<TransactionSpec> {
    fn into_specs(self) -> Result<Vec<TransactionSpec>, Error> {
        Ok(self)
    }
}

impl IntoFilterSpecs for Result<Vec<TransactionSpec>, Error> {
    fn into_specs(self) -> Result<Vec<TransactionSpec>, Error> {
        self
    }
}

/// Built-in facet that can rewrite, replace, or drop [`TransactionSpec`]s before they are applied.
///
/// Unlike [`ChangeFilter`] (which only masks document changes), a `TransactionFilter` can
/// inspect a preview [`Transaction`] and return a replacement list of [`TransactionSpec`]s
/// (including `vec![tr.spec().clone()]` to keep the original spec plus additional specs).
///
/// # Examples
///
/// ```
/// use texera_core::{State, TransactionFilter, tr};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Block any transaction that would make the document longer than 5 characters:
/// let state = State::from("hello").with_extensions(TransactionFilter::new(|tr| {
///     if tr.doc().len_chars() > 5 {
///         vec![]
///     } else {
///         vec![tr.spec().clone()]
///     }
/// }));
///
/// let blocked = state.clone().update(tr!({ changes: { from: 5, insert: "!" } }))?;
/// assert_eq!(blocked.state_ref().doc().to_string(), "hello");
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct TransactionFilter(pub Arc<dyn Fn(&Transaction) -> Result<Vec<TransactionSpec>, Error> + Send + Sync>);

impl TransactionFilter {
    /// Registers a transaction filter closure as an [`Extension`].
    pub fn new<F, R>(f: F) -> Extension
    where
        F: Fn(&Transaction) -> R + Send + Sync + 'static,
        R: IntoFilterSpecs,
    {
        Extension::from(Self(Arc::new(move |tr| f(tr).into_specs())))
    }
}

impl Facet for TransactionFilter {
    type Mode = Multi;
}

/// Built-in facet that appends extra [`Annotation`](crate::Annotation)s or
/// [`StateEffect`](crate::StateEffect)s to a transaction after changes and filters have been resolved.
///
/// Extenders run in reverse registration order (matching CodeMirror 6) and cannot modify
/// document changes or selection, making them lightweight hooks for tagging transactions
/// or emitting side-channel effects.
///
/// # Examples
///
/// ```
/// use texera_core::{State, TransactionExtender, StateEffect, Effect, tr};
///
/// #[derive(Clone, Debug, PartialEq)]
/// struct DocModifiedEffect;
/// impl Effect for DocModifiedEffect {}
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let state = State::from("hi").with_extensions(TransactionExtender::new(|tr| {
///     if tr.doc_changed() {
///         tr!({ effects: [StateEffect::new(DocModifiedEffect)] })
///     } else {
///         tr!()
///     }
/// }));
///
/// let res = state.update(tr!({ changes: { from: 2, insert: "!" } }))?;
/// assert!(res.effects().iter().any(|e| e.is::<DocModifiedEffect>()));
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct TransactionExtender(pub Arc<dyn Fn(&Transaction) -> TransactionSpec + Send + Sync>);

impl TransactionExtender {
    /// Registers a transaction extender closure as an [`Extension`].
    pub fn new<F>(f: F) -> Extension
    where
        F: Fn(&Transaction) -> TransactionSpec + Send + Sync + 'static,
    {
        Extension::from(Self(Arc::new(f)))
    }
}

impl Facet for TransactionExtender {
    type Mode = Multi;
}


impl State {
    #[inline]
    pub(crate) fn get_slot(&self, addr: SlotAddr) -> &Arc<AnySync> {
        match addr {
            SlotAddr::Static(idx) => &self.config.static_values[idx as usize],
            SlotAddr::Dynamic(idx) => &self.dynamic_slots[idx as usize],
        }
    }
    
    /// Queries the combined output value of facet `F` in this state.
    ///
    /// If no providers for `F` were registered in the state's extensions, returns
    /// `F::combine(&[])` (the facet's empty/default output).
    ///
    /// # Examples
    ///
    /// ```
    /// use texera_core::{State, TabSize};
    ///
    /// let state = State::from("code").with_extensions(TabSize(2));
    /// assert_eq!(state.facet::<TabSize>(), 2);
    /// ```
    pub fn facet<F: FacetDef>(&self) -> F::Output {
        match self.config.address.get(&TypeId::of::<F>()) {
            Some(&addr) => self
                .get_slot(addr)
                .downcast_ref::<F::Output>()
                .expect("Facet type mismatch")
                .clone(),
            None => F::combine(&[]),
        }
    }

    /// Queries the current value of [`StateField`] `F`, returning `None` if `F` is not
    /// registered in the state's configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use texera_core::{State, StateField, Transaction};
    ///
    /// struct CharCount;
    /// impl StateField for CharCount {
    ///     type Value = usize;
    ///     fn create(state: &State) -> usize { state.doc().len_chars() }
    ///     fn update(_val: &usize, tr: &Transaction) -> usize { tr.doc().len_chars() }
    /// }
    ///
    /// let state = State::from("hello").with_extensions(CharCount::init());
    /// assert_eq!(state.field::<CharCount>(), Some(&5));
    /// ```
    pub fn field<F: StateField>(&self) -> Option<&F::Value> {
        let addr = *self.config.address.get(&TypeId::of::<F>())?;
        Some(
            self.get_slot(addr)
                .downcast_ref::<F::Value>()
                .expect("StateField type mismatch"),
        )
    }

    /// Returns the configured [`TabSize`] (defaults to `4`).
    pub fn tab_size(&self) -> usize {
        self.facet::<TabSize>()
    }

    /// Returns `true` if the [`ReadOnly`] facet is enabled on this state.
    pub fn read_only(&self) -> bool {
        self.facet::<ReadOnly>()
    }

    /// Returns the configured [`LineSeparator`] string, or `"\n"` if none is set.
    ///
    /// # Examples
    ///
    /// ```
    /// use texera_core::{State, LineSeparator};
    ///
    /// let default_state = State::from("a\nb");
    /// assert_eq!(default_state.line_break(), "\n");
    ///
    /// let crlf_state = State::from("a\nb").with_extensions(LineSeparator("\r\n"));
    /// assert_eq!(crlf_state.line_break(), "\r\n");
    /// ```
    pub fn line_break(&self) -> &'static str {
        self.facet::<LineSeparator>().unwrap_or("\n")
    }

    /// Extracts a substring of the document between character offsets `from` (default `0`)
    /// and `to` (default `doc.len_chars()`), joining lines with the state's configured
    /// [`LineSeparator`].
    ///
    /// # Examples
    ///
    /// ```
    /// use texera_core::{State, LineSeparator};
    ///
    /// let state = State::from("hello\nworld").with_extensions(LineSeparator("\r\n"));
    /// assert_eq!(state.slice_doc(Some(3), Some(8)), "lo\r\nwo");
    /// assert_eq!(state.slice_doc(None, None), "hello\r\nworld");
    /// ```
    pub fn slice_doc(&self, from: Option<usize>, to: Option<usize>) -> String {
        let sep = self.facet::<LineSeparator>().map(str::to_string);
        self.doc.slice_string(from.unwrap_or(0), to, sep)
    }

    /// Returns a [`CharCategorizer`] configured with the [`WordChars`] registered in this state.
    ///
    /// # Examples
    ///
    /// ```
    /// use texera_core::{State, WordChars, CharCategory};
    ///
    /// let state = State::from("$var").with_extensions(WordChars::new("$"));
    /// let cat = state.char_categorizer(0);
    /// assert_eq!(cat.of('$'), CharCategory::Word);
    /// ```
    pub fn char_categorizer(&self, _at: usize) -> CharCategorizer {
        CharCategorizer::new(self.facet::<WordChars>())
    }

    /// Finds the word surrounding character offset `pos` in $O(N)$ time over the line,
    /// honoring custom [`WordChars`] and full Unicode grapheme clusters.
    ///
    /// Returns `None` if no word characters are adjacent to `pos`.
    ///
    /// # Examples
    ///
    /// ```
    /// use texera_core::{State, WordChars};
    ///
    /// let state = State::from("fn my_func() {}").with_extensions(WordChars::new("_"));
    /// let range = state.word_at(5).unwrap();
    /// assert_eq!((range.from(), range.to()), (3, 10));
    /// assert_eq!(state.slice_doc(Some(range.from()), Some(range.to())), "my_func");
    ///
    /// // Non-word position surrounded by spaces/punctuation:
    /// assert!(state.word_at(12).is_none());
    /// ```
    pub fn word_at(&self, pos: usize) -> Option<SelectionRange> {
        let line = self.doc.line_at(pos).ok()?;
        let text = line.text();
        let from = line.from();
        let length = line.length();
        let cat = self.char_categorizer(pos);

        let target = pos.saturating_sub(from).min(length);

        if text.is_ascii() {
            let bytes = text.as_bytes();
            let mut start = target;
            let mut end = target;

            let mut i = target;
            while i > 0 {
                let prev = &text[i - 1..i];
                if cat.categorize(prev) != CharCategory::Word {
                    break;
                }
                i -= 1;
                start = i;
            }

            let mut j = target;
            while j < bytes.len() {
                let next = &text[j..j + 1];
                if cat.categorize(next) != CharCategory::Word {
                    break;
                }
                j += 1;
                end = j;
            }

            return if start == end {
                None
            } else {
                Some(Selection::range(start + from, end + from))
            };
        }

        let mut graphemes: Vec<(usize, usize, &str)> = Vec::new();
        let mut char_idx = 0;
        let mut split_idx = 0;

        for g in text.graphemes(true) {
            let next_idx = char_idx + g.chars().count();
            if next_idx <= target {
                split_idx = graphemes.len() + 1;
            }
            graphemes.push((char_idx, next_idx, g));
            char_idx = next_idx;
        }

        let mut start = target;
        let mut end = target;

        for &(g_from, _, g_str) in graphemes[..split_idx].iter().rev() {
            if cat.categorize(g_str) != CharCategory::Word {
                break;
            }
            start = g_from;
        }

        for &(_, g_to, g_str) in &graphemes[split_idx..] {
            if cat.categorize(g_str) != CharCategory::Word {
                break;
            }
            end = g_to;
        }

        if start == end {
            None
        } else {
            Some(Selection::range(start + from, end + from))
        }
    }

    /// Converts a string slice into a [`Text`] rope, normalizing line breaks according to
    /// the state's configured [`LineSeparator`].
    ///
    /// If a custom [`LineSeparator`] (e.g. `"\r\n"`) is configured, only occurrences of that
    /// separator become document line breaks, while standalone `'\n'` characters are replaced
    /// with spaces (matching CodeMirror 6's `EditorState.toText`).
    ///
    /// # Examples
    ///
    /// ```
    /// use texera_core::{State, LineSeparator};
    ///
    /// let state = State::from("").with_extensions(LineSeparator("\r\n"));
    /// let text = state.to_text("line1\r\nline2");
    /// assert_eq!(text.len_lines(), 2);
    /// ```
    pub fn to_text(&self, text: &str) -> Text {
        match self.facet::<LineSeparator>() {
            Some(sep) => {
                let cleaned = if sep != "\n" {
                    let mut result = String::with_capacity(text.len());
                    let mut i = 0;
                    let sep_bytes = sep.as_bytes();
                    let text_bytes = text.as_bytes();
                    while i < text_bytes.len() {
                        if text_bytes[i..].starts_with(sep_bytes) {
                            result.push('\n');
                            i += sep_bytes.len();
                        } else if text_bytes[i] == b'\n' {
                            result.push(' ');
                            i += 1;
                        } else {
                            let start = i;
                            while i < text_bytes.len()
                                && !text_bytes[i..].starts_with(sep_bytes)
                                && text_bytes[i] != b'\n'
                            {
                                i += 1;
                            }
                            if let Ok(s) = std::str::from_utf8(&text_bytes[start..i]) {
                                result.push_str(s);
                            }
                        }
                    }
                    result
                } else {
                    text.to_string()
                };
                Text::from(cleaned.as_str())
            }
            None => Text::from(text),
        }
    }
    
    /// Evaluates a group of static facet providers against `state` and combines their outputs.
    pub fn eval_facet(group: &[Arc<ErasedFacetProvider>], state: &State) -> Arc<AnySync> {
        let raw_vals: Vec<Arc<AnySync>> = group.iter().map(|p| (p.eval)(state)).collect();
        (group[0].compute_static)(&raw_vals)
    }
}
