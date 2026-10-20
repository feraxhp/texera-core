//! Multi-cursor selections, selection ranges, and position association.
//!
//! This module provides editor selection primitives with automatic normalization:
//!
//! - [`Selection`]: An immutable collection of non-overlapping, sorted [`SelectionRange`]s
//!   with a designated primary (`main`) range.
//! - [`SelectionRange`]: A directional text range bounded by an [`anchor`](SelectionRange::anchor)
//!   and a cursor [`head`](SelectionRange::head), supporting goal columns and associativity.
//! - [`Assoc`]: Bias indicating which side of an inserted character a boundary position attaches to.
//! - [`SelectionSpec`]: Flexible input type allowing constructors to accept cursors, ranges, or full selections.

pub mod selection;
pub mod range;
mod structs;
mod tests;

pub use self::selection::SelectionIntoIter;
pub use self::structs::*;
