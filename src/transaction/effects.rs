use std::any::TypeId;
use std::sync::Arc;
use std::any::Any;

use crate::extension::{AppendConfigEffect, ReconfigureEffect};
use crate::{AnySync, ChangeDesc, Extension};

/// Trait implemented by custom state effect payloads that can be dispatched inside a
/// [`Transaction`](crate::Transaction) via [`StateEffect`].
///
/// Effects are used to communicate state updates to custom [`StateField`](crate::StateField)s
/// (for example, adding/removing diagnostics, folding code ranges, or toggling UI panels).
/// If an effect stores document positions, override [`Effect::map`] so its positions are
/// automatically adjusted when combined with document edits.
///
/// # Examples
///
/// ```rust
/// use texera_core::{Assoc, ChangeDesc, ChangeSet, ChangeSpec, Effect, MapMode, StateEffect};
///
/// #[derive(Clone, Debug, PartialEq, Eq)]
/// struct BookmarkAt(usize);
///
/// impl Effect for BookmarkAt {
///     fn map(&self, changes: &ChangeDesc) -> Option<Self> {
///         changes
///             .map_pos(self.0, Assoc::After, MapMode::TrackDel)
///             .ok()
///             .flatten()
///             .map(BookmarkAt)
///     }
/// }
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let eff = StateEffect::new(BookmarkAt(3));
/// let cs = ChangeSet::from_slice(&[ChangeSpec::new(0, None, Some("ab".into()))?], 10)?;
/// let mapped = eff.map(cs.description()).unwrap();
/// assert_eq!(mapped.get::<BookmarkAt>(), Some(&BookmarkAt(5)));
/// # Ok(())
/// # }
/// ```
pub trait Effect: Any + Send + Sync + Clone {
    /// Maps this effect through document `changes`. Returning `None` drops the effect (for example,
    /// if the range it refers to was deleted).
    fn map(&self, _changes: &ChangeDesc) -> Option<Self> {
        Some(self.clone())
    }
}

/// A type-erased wrapper around an [`Effect`] payload, capable of mapping itself through a
/// [`ChangeDesc`].
///
/// Also provides built-in configuration effects [`StateEffect::reconfigure`] and
/// [`StateEffect::append_config`].
#[derive(Clone, Debug)]
pub struct StateEffect {
    type_id: TypeId,
    value: Arc<AnySync>,
    map_fn: fn(&dyn Any, &ChangeDesc) -> Option<Arc<AnySync>>,
}

impl StateEffect {
    /// Wraps a typed [`Effect`] value into a [`StateEffect`].
    pub fn new<E: Effect>(effect: E) -> Self {
        Self {
            type_id: effect.type_id(),
            value: Arc::new(effect),
            map_fn: |any_val, changes| {
                let typed = any_val.downcast_ref::<E>().expect("StateEffect invariant violated: value always should match E");
                Some(Arc::new(typed.map(changes)?))
            },
        }
    }

    /// Creates a built-in effect that replaces the entire top-level [`Extension`] configuration of
    /// a [`State`](crate::State) while preserving existing [`Compartment`](crate::Compartment) overrides
    /// and [`StateField`](crate::StateField) values that remain present.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::{State, StateEffect, TabSize, tr};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let state = State::default();
    /// assert_eq!(state.tab_size(), 4);
    ///
    /// let state = state.update(tr!({
    ///     effects: [StateEffect::reconfigure(TabSize(2))],
    /// }))?.into_state();
    /// assert_eq!(state.tab_size(), 2);
    /// # Ok(())
    /// # }
    /// ```
    pub fn reconfigure(ext: impl Into<Extension>) -> Self {
        Self::new(ReconfigureEffect(ext.into()))
    }

    /// Creates a built-in effect that appends `ext` to the end of the state's current top-level
    /// [`Extension`] configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::{ReadOnly, State, StateEffect, tr};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let state = State::default();
    /// assert!(!state.read_only());
    ///
    /// let state = state.update(tr!({
    ///     effects: [StateEffect::append_config(ReadOnly(true))],
    /// }))?.into_state();
    /// assert!(state.read_only());
    /// # Ok(())
    /// # }
    /// ```
    pub fn append_config(ext: impl Into<Extension>) -> Self {
        Self::new(AppendConfigEffect(ext.into()))
    }
    
    /// Returns `true` if this effect holds a payload of type `E`.
    #[inline]
    pub fn is<E: Effect>(&self) -> bool { self.type_id == TypeId::of::<E>() }
    
    /// Downcasts and returns a reference to the underlying effect `E` if the type matches.
    #[inline]
    pub fn get<E: Effect>(&self) -> Option<&E> {
        match self.type_id == TypeId::of::<E>() {
            true => self.value.downcast_ref::<E>(),
            false => None,
        }
    }
    
    /// Maps this effect through `changes` using [`Effect::map`].
    pub fn map(&self, changes: &ChangeDesc) -> Option<StateEffect> {
        Some(StateEffect {
            type_id: self.type_id,
            value: (self.map_fn)(self.value.as_ref(), changes)?,
            map_fn: self.map_fn,
        })
    }
}
