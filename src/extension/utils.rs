use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::any::TypeId;


static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
pub(crate) fn next_id() -> usize { NEXT_ID.fetch_add(1, Ordering::Relaxed) }


/// Returns the `TypeId` of `T` for declaring dynamic facet dependencies in
/// [`FacetDef::compute`](crate::FacetDef::compute) and [`FacetDef::compute_multi`](crate::FacetDef::compute_multi).
///
/// Supported dependency targets `T`:
/// - [`Text`](crate::Text): recompute whenever the document changes.
/// - [`Selection`](crate::Selection): recompute whenever the document or selection changes.
/// - Any [`StateField`](crate::StateField) type: recompute whenever that state field updates.
/// - Any [`FacetDef`](crate::FacetDef) type: recompute whenever that upstream facet's value changes.
///
/// # Examples
///
/// ```rust
/// use texera_core::{Facet, FacetDef, Multi, State, Text, dep};
///
/// #[derive(Clone, Debug, PartialEq, Eq)]
/// struct DocLen(usize);
/// impl Facet for DocLen {
///     type Mode = Multi;
///     fn compare_input(a: &Self, b: &Self) -> bool { a == b }
/// }
///
/// let state = State::from("hello").with_extensions(
///     DocLen::compute([dep::<Text>()], |s| DocLen(s.doc().len_chars())),
/// );
/// assert_eq!(state.facet::<DocLen>(), vec![DocLen(5)]);
/// ```
#[inline]
pub fn dep<T: 'static>() -> TypeId {
    TypeId::of::<T>()
}
