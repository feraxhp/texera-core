use std::any::{Any, TypeId};
use std::sync::Arc;

use crate::{Extension, State, Transaction};

use super::ErasedStateField;


/// Custom state slot stored inside [`State`] and updated transactionally via [`StateField::update`].
///
/// Unlike facets (which are derived from inputs), a `StateField` maintains its own persistent
/// state across transactions and reconfigurations, and can optionally provide values to facets via
/// [`StateField::provide`].
///
/// # Examples
///
/// ```rust
/// use texera_core::{Extension, Facet, FacetDef, Multi, State, StateField, Transaction, tr};
///
/// #[derive(Clone, Debug, PartialEq, Eq)]
/// struct StatusBadge(String);
/// impl Facet for StatusBadge {
///     type Mode = Multi;
///     fn compare_input(a: &Self, b: &Self) -> bool { a == b }
/// }
///
/// struct LineCounter;
/// impl StateField for LineCounter {
///     type Value = usize;
///
///     fn create(state: &State) -> usize {
///         state.doc().len_lines()
///     }
///     fn update(val: &usize, tr: &Transaction) -> usize {
///         if tr.doc_changed() { tr.doc().len_lines() } else { *val }
///     }
///     fn compare(a: &usize, b: &usize) -> bool { a == b }
///     fn provide() -> Option<Extension> {
///         Some(StatusBadge::from_field_with::<Self>(|n| StatusBadge(format!("lines:{n}"))))
///     }
/// }
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let state = State::from("a\nb").with_extensions(LineCounter::init());
/// assert_eq!(state.field::<LineCounter>(), Some(&2));
/// assert_eq!(state.facet::<StatusBadge>(), vec![StatusBadge("lines:2".into())]);
///
/// let state = state.update(tr!({ changes: { from: 3, insert: "\nc" } }))?.into_state();
/// assert_eq!(state.field::<LineCounter>(), Some(&3));
/// assert_eq!(state.facet::<StatusBadge>(), vec![StatusBadge("lines:3".into())]);
/// # Ok(())
/// # }
/// ```
pub trait StateField: Any + Send + Sync + 'static {
    /// Type of the value stored in the state slot.
    type Value: Clone + Send + Sync + 'static;

    /// Initializes the field's value when the state is created or the field is first added.
    fn create(state: &State) -> Self::Value;

    /// Computes the field's next value given its previous `value` and the applied [`Transaction`].
    fn update(value: &Self::Value, tx: &Transaction) -> Self::Value;

    /// Compares two field values for equality. Returning `true` preserves the existing `Arc` pointer
    /// and prevents unnecessary recomputation of downstream facets that depend on this field.
    fn compare(_a: &Self::Value, _b: &Self::Value) -> bool {
        false
    }

    /// Optional hook returning an [`Extension`] (such as [`FacetDef::from_field`](crate::FacetDef::from_field))
    /// that is automatically registered whenever this field is added to a [`State`].
    fn provide() -> Option<Extension>
    where
        Self: Sized,
    {
        None
    }

    /// Returns an [`Extension`] that registers this `StateField` using [`StateField::create`].
    fn init() -> Extension
    where
        Self: Sized,
    {
        Extension::state_field::<Self>()
    }

    /// Returns an [`Extension`] that registers this `StateField` using a custom initializer closure.
    fn init_with(
        create: impl Fn(&State) -> Self::Value + Send + Sync + 'static,
    ) -> Extension
    where
        Self: Sized,
    {
        Extension::state_field_with::<Self>(create)
    }
}

impl Extension {
    /// Registers a [`StateField`] `F` using its default [`StateField::create`] constructor.
    pub fn state_field<F: StateField>() -> Extension {
        Self::state_field_with::<F>(F::create)
    }

    /// Registers a [`StateField`] `F` with a custom `create` closure.
    pub fn state_field_with<F: StateField>(
        create: impl Fn(&State) -> F::Value + Send + Sync + 'static,
    ) -> Extension {
        Extension::StateField(ErasedStateField {
            type_id: TypeId::of::<F>(),
            create_fn: Arc::new(move |state| Arc::new(create(state))),
            update_fn: |old_any, tx| {
                let old_val = old_any
                    .downcast_ref::<F::Value>()
                    .expect("StateField type mismatch");
                let new_val = F::update(old_val, tx);
                if F::compare(old_val, &new_val) {
                    Arc::clone(old_any)
                } else {
                    Arc::new(new_val)
                }
            },
            provide: F::provide().map(Box::new),
        })
    }
}
