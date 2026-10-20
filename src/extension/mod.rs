//! Type-safe extension system, facets, state fields, and dynamic reconfiguration.
//!
//! This module provides the extensible plugin architecture ported from `@codemirror/state`:
//!
//! - [`Extension`]: A tree or collection of configuration values, facets, and state fields
//!   attached to a [`State`](crate::State).
//! - [`Facet`] & [`FacetDef`]: Extension points that aggregate inputs into computed values
//!   ([`Single`] or [`Multi`]), supporting dependency ordering via [`dep`].
//! - [`StateField`]: Persistent state storage that lives within the editor state, initialized
//!   via [`StateField::create`] and updated deterministically on transactions via [`StateField::update`].
//! - [`Compartment`]: Dynamic containers that allow replacing a subtree of extensions at runtime
//!   without re-creating the entire editor state.
//! - [`Priority`]: Explicit ordering levels (`Highest`, `High`, `Default`, `Low`, `Lowest`) for resolving
//!   extension precedence.

pub(crate) mod utils;
pub mod priority;
pub mod flatten;
pub mod state_field;
pub mod from;
mod structs;
mod traits;
mod facet;
mod tests;

pub use self::state_field::StateField;
pub use self::structs::*;
pub use self::traits::*;
pub use self::facet::*;
pub use self::utils::dep;
