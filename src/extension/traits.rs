/// Convenience trait for converting a [`FacetDef`](crate::FacetDef) input value directly into an
/// [`Extension`](crate::Extension).
///
/// Automatically implemented for all types `F` where `F: FacetDef<Input = F>` (including all
/// types implementing [`Facet`](crate::Facet)).
///
/// # Examples
///
/// ```rust
/// use texera_core::{IntoExtension, State, TabSize};
///
/// let state = State::from(TabSize(8).into_ext());
/// assert_eq!(state.tab_size(), 8);
/// ```
pub trait IntoExtension {
    /// Wraps `self` as a static facet provider [`Extension`](crate::Extension).
    fn into_ext(self) -> super::Extension;
}
