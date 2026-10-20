//! Layered collections of intervals, markers, code folding ranges, and decorations.
//!
//! This module implements chunked B-tree storage of intervals associated with custom [`RangeValue`] items:
//!
//! - [`RangeSet`]: Persistent, immutable container of ranges. Non-overlapping ranges are stored
//!   in 250-element leaf [`Chunk`]s, while overlapping ranges automatically spill into secondary layers.
//! - [`RangeSetBuilder`]: High-performance builder for constructing a [`RangeSet`] from sorted ranges.
//! - [`RangeValue`]: Trait describing custom range behavior (associativity, map mode, point status).
//! - [`RangeSet::compare`]: $O(1)$-skipping algorithm for diffing two sets of decorations across document edits.
//! - [`RangeSetCursor`]: Min-heap cursor for iterating across multiple sets and layers in order.
//! - [`SpanIterator`]: Visitor callback for partitioning intervals into uniform active spans.

pub mod range_set;
pub mod builder;
pub mod compare;
pub mod chunk;
mod layer_cursor;
mod range_cursor;
mod span_cursor;
mod structs;
mod traits;
mod utils;
mod tests;

pub use self::structs::*;
pub use self::traits::*;
