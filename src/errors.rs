//! Error categories, codes, and constructors for invalid editor states and transitions.

use std::fmt::Display;

use mynox::Formater;
use mynox::error;
use mynox::Error;


/// Canonical error categories emitted by `texera` operations.
///
/// Each variant provides a stable string identifier via [`ErrorTypes::get_etype`] that can be
/// matched against `mynox::Error::etype()`.
///
/// # Examples
///
/// ```rust
/// use texera_core::{ChangeSpec, ErrorTypes};
///
/// let err = ChangeSpec::new(10, Some(5), None).unwrap_err();
/// assert_eq!(err.etype(), ErrorTypes::RangeError.get_etype());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorTypes {
    /// An index or position exceeded the bounds of the document or collection.
    OutOfBounds,
    /// A range specification was invalid (e.g. `from > to`, or out-of-order ranges).
    RangeError,
    /// Two changes in the same transaction overlapped illegally.
    TransactionOverlap,
    /// Attempted to compose or map two [`ChangeDesc`](crate::ChangeDesc) / [`ChangeSet`](crate::ChangeSet)
    /// instances whose document lengths do not match.
    DifferingChanges,
    /// The same [`Compartment`](crate::Compartment) was registered more than once in a [`State`](crate::State).
    DuplicateCompartment,
}

impl ErrorTypes {
    /// Returns the static error type code string associated with this variant.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::ErrorTypes;
    ///
    /// assert_eq!(
    ///     ErrorTypes::DuplicateCompartment.get_etype(),
    ///     "texera::state::compartment::duplicate"
    /// );
    /// ```
    pub fn get_etype(self) -> &'static str {
        match self {
            ErrorTypes::OutOfBounds => "texera::state::text::index::out_of_range",
            ErrorTypes::RangeError => "texera::state::invalid::range",
            ErrorTypes::TransactionOverlap => "texera::state::transaction::overlap",
            ErrorTypes::DifferingChanges => "texera::state::diff::changes",
            ErrorTypes::DuplicateCompartment => "texera::state::compartment::duplicate",
        }
    }
}

pub(crate) fn out_of_bounds(idx: usize, maximun: usize) -> Error {
    error!(
        ErrorTypes::OutOfBounds.get_etype(),
        "Index out of bounds",
        format!("The given index can not be greater than `{maximun}`"),
        => format!("given index: **{idx}**")
    )
}

pub(crate) fn range_error<D: Display>(tip: D) -> Error {
    error!(
        ErrorTypes::RangeError.get_etype(),
        "Invalid range",
        "You are trying to create an invalid range",
        => tip.as_tip()
    )
}

#[allow(dead_code)]
pub(crate) fn transaction_overlap<D: Display>(tip: D) -> Error {
    error!(
        ErrorTypes::TransactionOverlap.get_etype(),
        "Change set overlap detected",
        "Two _ChangeSpec_ cant overlap in the same transaction",
        => tip.as_tip()
    )
}

pub(crate) fn differing_changes<D: Display>(description: D) -> Error {
    error!(
        ErrorTypes::DifferingChanges.get_etype(),
        "Invalid pair of changes",
        description
    )
}

pub(crate) fn duplicate_compartment() -> Error {
    error!(
        ErrorTypes::DuplicateCompartment.get_etype(),
        "Duplicate use of compartment",
        "A compartment cannot be used more than once in the same state configuration"
    )
}
