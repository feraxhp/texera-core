/// Classification category assigned to characters or grapheme clusters.
///
/// Used when finding word boundaries or grouping adjacent characters during
/// word-wise cursor movement and double-click selection.
///
/// # Examples
///
/// ```
/// use texera_core::CharCategory;
///
/// assert_eq!(CharCategory::from('a'), CharCategory::Word);
/// assert_eq!(CharCategory::from('_'), CharCategory::Word);
/// assert_eq!(CharCategory::from(' '), CharCategory::Space);
/// assert_eq!(CharCategory::from('+'), CharCategory::Other);
/// ```
/// Classification category assigned to characters or grapheme clusters.
///
/// Used when finding word boundaries or grouping adjacent characters during
/// word-wise cursor movement and double-click selection.
///
/// # Examples
///
/// ```
/// use texera_core::CharCategory;
///
/// assert_eq!(CharCategory::from('a'), CharCategory::Word);
/// assert_eq!(CharCategory::from('_'), CharCategory::Word);
/// assert_eq!(CharCategory::from(' '), CharCategory::Space);
/// assert_eq!(CharCategory::from('+'), CharCategory::Other);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CharCategory {
    /// Alphanumeric characters, `_`, or custom characters configured via [`WordChars`].
    Word,
    /// Whitespace characters (`' '`, `'\t'`, `'\n'`, etc.).
    Space,
    /// Punctuation, symbols, and any other non-word, non-whitespace characters.
    Other,
}

/// Built-in facet that registers additional characters to be treated as
/// [`CharCategory::Word`] (for example, `"$"` in JavaScript/PHP or `"-"` in CSS).
///
/// Multiple [`WordChars`] inputs registered across extensions are concatenated
/// and deduplicated automatically.
///
/// # Examples
///
/// ```
/// use texera_core::{State, WordChars};
///
/// let state = State::from("let $foo_bar = 1;")
///     .with_extensions(WordChars::new("$"));
///
/// let word = state.word_at(5).unwrap();
/// assert_eq!(state.slice_doc(Some(word.from()), Some(word.to())), "$foo_bar");
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WordChars(pub String);

/// Classifies characters and strings into [`CharCategory`] buckets, honoring any
/// custom word characters.
///
/// Obtain one configured for a [`State`](crate::State) via
/// [`State::char_categorizer`](crate::State::char_categorizer), or build a standalone
/// categorizer with [`CharCategorizer::new`] or [`make_categorizer`](crate::make_categorizer).
///
/// # Examples
///
/// ```
/// use texera_core::{CharCategorizer, CharCategory};
///
/// let cat = CharCategorizer::new("$-");
/// assert_eq!(cat.of('$'), CharCategory::Word);
/// assert_eq!(cat.of('-'), CharCategory::Word);
/// assert_eq!(cat.of('.'), CharCategory::Other);
/// assert_eq!(cat.of(' '), CharCategory::Space);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CharCategorizer {
    pub(crate) word_chars: String,
}
