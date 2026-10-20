use crate::extension::FacetDef;
use crate::Extension;

use super::CharCategorizer;
use super::CharCategory;
use super::WordChars;


pub(crate) fn has_word_char(c: &char) -> bool {
    c.is_alphabetic() || c.is_numeric() || c == &'_'
}

impl From<&char> for CharCategory {
    fn from(value: &char) -> Self {
        match () {
            _ if value.is_whitespace() => Self::Space,
            _ if has_word_char(value) => Self::Word,
            _ => Self::Other,
        }
    }
}

impl From<char> for CharCategory {
    fn from(value: char) -> Self { Self::from(&value) }
}

impl WordChars {
    /// Creates a [`WordChars`] facet input containing the specified custom word characters.
    ///
    /// # Examples
    ///
    /// ```
    /// use texera_core::{State, WordChars};
    ///
    /// let state = State::from("my-css-var").with_extensions(WordChars::new("-"));
    /// let word = state.word_at(3).unwrap();
    /// assert_eq!((word.from(), word.to()), (0, 10));
    /// ```
    pub fn new(chars: impl Into<String>) -> Self {
        Self(chars.into())
    }
}

impl From<&str> for WordChars {
    fn from(value: &str) -> Self { Self(value.to_string()) }
}

impl From<String> for WordChars {
    fn from(value: String) -> Self { Self(value) }
}

impl FacetDef for WordChars {
    type Input = WordChars;
    type Output = String;

    fn combine(inputs: &[Self::Input]) -> String {
        let mut out = String::new();
        for input in inputs {
            for c in input.0.chars() {
                if !out.contains(c) {
                    out.push(c);
                }
            }
        }
        out
    }

    fn compare_input(a: &Self::Input, b: &Self::Input) -> bool { a == b }
    fn compare(a: &String, b: &String) -> bool { a == b }
}

impl From<WordChars> for Extension {
    fn from(value: WordChars) -> Self {
        Extension::facet::<WordChars>(value)
    }
}

/// Trait implemented by types (`char`, `str`, `String`) that can be classified by a [`CharCategorizer`].
pub trait CategorizeInput {
    /// Classifies `self` into a [`CharCategory`] using `word_chars` as additional word characters.
    fn categorize_with(&self, word_chars: &str) -> CharCategory;
}

impl CategorizeInput for char {
    fn categorize_with(&self, word_chars: &str) -> CharCategory {
        if self.is_whitespace() {
            CharCategory::Space
        } else if has_word_char(self) || word_chars.contains(*self) {
            CharCategory::Word
        } else {
            CharCategory::Other
        }
    }
}

impl CategorizeInput for &char {
    fn categorize_with(&self, word_chars: &str) -> CharCategory {
        (**self).categorize_with(word_chars)
    }
}

impl CategorizeInput for str {
    fn categorize_with(&self, word_chars: &str) -> CharCategory {
        if !self.chars().any(|c| !c.is_whitespace()) {
            return CharCategory::Space;
        }
        if self.chars().any(|c| has_word_char(&c) || word_chars.contains(c)) {
            return CharCategory::Word;
        }
        CharCategory::Other
    }
}

impl CategorizeInput for &str {
    fn categorize_with(&self, word_chars: &str) -> CharCategory {
        (*self).categorize_with(word_chars)
    }
}

impl CategorizeInput for String {
    fn categorize_with(&self, word_chars: &str) -> CharCategory {
        self.as_str().categorize_with(word_chars)
    }
}

impl CharCategorizer {
    /// Creates a new [`CharCategorizer`] that treats any character in `word_chars` as [`CharCategory::Word`].
    ///
    /// # Examples
    ///
    /// ```
    /// use texera_core::{CharCategorizer, CharCategory};
    ///
    /// let cat = CharCategorizer::new("@");
    /// assert_eq!(cat.of('@'), CharCategory::Word);
    /// ```
    pub fn new(word_chars: impl Into<String>) -> Self {
        Self { word_chars: word_chars.into() }
    }

    /// Categorizes a borrowed character or string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use texera_core::{CharCategorizer, CharCategory};
    ///
    /// let cat = CharCategorizer::new("$");
    /// assert_eq!(cat.categorize("hello"), CharCategory::Word);
    /// assert_eq!(cat.categorize("$"), CharCategory::Word);
    /// assert_eq!(cat.categorize("   "), CharCategory::Space);
    /// assert_eq!(cat.categorize("=>"), CharCategory::Other);
    /// ```
    pub fn categorize<I: CategorizeInput + ?Sized>(&self, input: &I) -> CharCategory {
        input.categorize_with(&self.word_chars)
    }

    /// Categorizes a character or string passed by value.
    ///
    /// # Examples
    ///
    /// ```
    /// use texera_core::{CharCategorizer, CharCategory};
    ///
    /// let cat = CharCategorizer::new("");
    /// assert_eq!(cat.of('7'), CharCategory::Word);
    /// assert_eq!(cat.of('\t'), CharCategory::Space);
    /// ```
    pub fn of<I: CategorizeInput>(&self, input: I) -> CharCategory {
        input.categorize_with(&self.word_chars)
    }
}

/// Convenience function that creates a [`CharCategorizer`] with the given additional word characters.
///
/// # Examples
///
/// ```
/// use texera_core::{make_categorizer, CharCategory};
///
/// let cat = make_categorizer("$");
/// assert_eq!(cat.of('$'), CharCategory::Word);
/// assert_eq!(cat.of('#'), CharCategory::Other);
/// ```
pub fn make_categorizer(word_chars: &str) -> CharCategorizer {
    CharCategorizer::new(word_chars)
}
