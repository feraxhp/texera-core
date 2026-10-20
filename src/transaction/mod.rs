//! Declarative state transitions, effects, annotations, and transaction specifications.
//!
//! This module implements the transactional update pipeline ported from `@codemirror/state`:
//!
//! - [`TransactionSpec`]: Declarative description of changes, selection updates, annotations,
//!   and effects to apply in a single state transition.
//! - [`tr!`]: Macro for ergonomically creating a [`TransactionSpec`].
//! - [`Transaction`]: Concrete evaluated transition object passed to [`StateField`](crate::StateField)s
//!   and transaction filters before finalizing the state update.
//! - [`StateEffect`]: Out-of-band communication channel used for reconfiguration ([`StateEffect::reconfigure`]),
//!   appending extensions ([`StateEffect::append_config`]), or custom extension-specific events.
//! - [`Annotation`]: Metadata tags attached to transactions (such as [`UserEvent`]) to identify the intent
//!   or source of an edit (e.g. keyboard typing, paste, undo/redo).

mod transaction_spec;
mod transaction;
mod anotations;
mod effects;

pub use self::transaction::*;
pub use self::anotations::*;
pub use self::effects::*;
pub use self::transaction_spec::TransactionSpec;
