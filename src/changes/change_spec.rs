use mynox::Error;
use mynox::Formater;

use crate::errors::range_error;
use crate::Text;


/// A declarative specification of a single document edit (insertion, deletion, or replacement).
///
/// Multiple `ChangeSpec` instances can be compiled into a normalized [`ChangeSet`](crate::ChangeSet)
/// via [`ChangeSet::from_slice`](crate::ChangeSet::from_slice) or passed directly into a
/// [`TransactionSpec`](crate::TransactionSpec) (or [`tr!`](crate::tr) / [`ch!`](crate::ch)).
///
/// # Examples
///
/// ```rust
/// use texera_core::{ChangeSet, ChangeSpec, Text};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Insert "Hello " at offset 0
/// let insert = ChangeSpec::new(0, None, Some(Text::from("Hello ")))?;
/// // Replace [0..5] with "World"
/// let replace = ChangeSpec::new(0, Some(5), Some(Text::from("World")))?;
/// // Delete [5..6]
/// let delete = ChangeSpec::new(5, Some(6), None)?;
///
/// let cs = ChangeSet::from_slice(&[insert, replace, delete], 6)?;
/// let result = cs.apply(Text::from("world!"))?;
/// assert_eq!(result.to_string(), "Hello World");
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeSpec { 
    pub(crate) from: usize, 
    pub(crate) to: usize, 
    pub(crate) insert: Option<Text>, 
}

impl PartialOrd for ChangeSpec {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ChangeSpec {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.from.cmp(&other.from).then(self.to.cmp(&other.to))
    }
}

impl ChangeSpec {
    /// Creates a new [`ChangeSpec`] spanning `[from..to]` and replacing it with `insert`.
    ///
    /// - If `to` is `None`, defaults to `from` (pure insertion).
    /// - If `insert` is `None`, defaults to empty text (pure deletion).
    ///
    /// Returns a [`RangeError`](crate::ErrorTypes::RangeError) if `from > to`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::ChangeSpec;
    ///
    /// assert!(ChangeSpec::new(2, Some(5), Some("abc".into())).is_ok());
    /// assert!(ChangeSpec::new(5, Some(2), None).is_err());
    /// ```
    pub fn new( from: usize, to: Option<usize>, insert: Option<Text> ) -> Result<Self, Error> {
        let to = to.unwrap_or(from);
        if from > to { 
            return Err(
                range_error("**from** should be less than **to**")
                    .extend_note(format!("from: `{from}`, to: `{to}`").as_tip_cotinuation())
            ) 
        };
        Ok(Self { from, to, insert })
    }

    /// Returns the start character offset in the original document.
    pub fn from(&self) -> usize { self.from }

    /// Returns the end character offset in the original document.
    pub fn to(&self) -> usize { self.to }

    /// Returns the optional inserted [`Text`].
    pub fn insert(&self) -> Option<&Text> { self.insert.as_ref() }
}
