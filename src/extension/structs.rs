/// Precedence tier for ordering [`Extension`]s.
///
/// Extensions wrapped with [`Extension::with_priority`] are evaluated in order from
/// [`Priority::Highest`] down to [`Priority::Lowest`]. Within the same priority tier, registration
/// order is preserved.
///
/// # Examples
///
/// ```rust
/// use texera_core::{Extension, Facet, IntoExtension, Multi, Priority, State};
///
/// #[derive(Clone, Debug, PartialEq, Eq)]
/// struct Order(&'static str);
/// impl Facet for Order {
///     type Mode = Multi;
///     fn compare_input(a: &Self, b: &Self) -> bool { a == b }
/// }
///
/// let state = State::from(Extension::from([
///     Order("default").into_ext(),
///     Order("low").into_ext().with_priority(Priority::Low),
///     Order("highest").into_ext().with_priority(Priority::Highest),
/// ]));
///
/// let vals: Vec<_> = state.facet::<Order>().into_iter().map(|o| o.0).collect();
/// assert_eq!(vals, vec!["highest", "default", "low"]);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Priority {
    /// Highest precedence tier (`0`), evaluated before all other tiers.
    Highest = 0,
    /// High precedence tier (`1`), evaluated before [`Priority::Default`].
    High = 1,
    /// Default precedence tier (`2`) when no explicit priority is attached.
    Default = 2,
    /// Low precedence tier (`3`), evaluated after [`Priority::Default`].
    Low = 3,
    /// Lowest fallback precedence tier (`4`), evaluated last.
    Lowest = 4,
}

use std::{any::TypeId, fmt::Debug, sync::Arc};

use crate::{AnySync, Effect, State, StateEffect, Transaction};

/// Type-erased representation of a static or dynamic facet provider.
pub struct ErasedFacetProvider {
    /// Unique identifier used to deduplicate identical providers and key `provider_cache`.
    pub id: usize,
    /// `TypeId` of the target [`FacetDef`](crate::FacetDef).
    pub facet_type_id: TypeId,
    /// `TypeId`s of state dependencies (`Text`, `Selection`, `StateField`s, or other `FacetDef`s).
    pub deps: Box<[TypeId]>,
    /// Evaluation closure that computes the provider's input value from a [`State`].
    pub eval: Box<dyn Fn(&State) -> Arc<AnySync> + Send + Sync>,
    /// Type-erased combiner function that merges all provider outputs into the facet's final value.
    pub compute_static: fn(&[Arc<AnySync>]) -> Arc<AnySync>,
    /// Type-erased equality comparator for individual provider inputs.
    pub compare_input: fn(&Arc<AnySync>, &Arc<AnySync>) -> bool,
    /// Type-erased equality comparator for the combined facet output.
    pub compare_output: fn(&Arc<AnySync>, &Arc<AnySync>) -> bool,
}

impl Debug for ErasedFacetProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ErasedFacetProvider")
            .field("id", &self.id)
            .field("facet_type_id", &self.facet_type_id)
            .field("deps", &self.deps)
            .field("eval", &"eval_function")
            .field("compute_static", &self.compute_static)
            .field("compare_input", &self.compare_input)
            .field("compare_output", &self.compare_output)
            .finish()
    }
}

/// Type-erased descriptor for a registered [`StateField`](crate::StateField).
#[derive(Clone)]
pub struct ErasedStateField {
    /// `TypeId` of the [`StateField`](crate::StateField) type.
    pub type_id: TypeId,
    /// Initializer closure invoked when the field is first added to a [`State`].
    pub create_fn: Arc<dyn Fn(&State) -> Arc<AnySync> + Send + Sync>,
    /// Transition function invoked on every [`Transaction`] to compute the next field value.
    pub update_fn: fn(&Arc<AnySync>, &Transaction) -> Arc<AnySync>,
    /// Optional extension(s) automatically provided alongside this [`StateField`](crate::StateField).
    pub provide: Option<Box<Extension>>,
}

impl Debug for ErasedStateField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ErasedStateField")
            .field("type_id", &self.type_id)
            .finish()
    }
}

/// A composable configuration node used to extend an editor [`State`].
///
/// Extensions can be single facet providers, custom [`StateField`](crate::StateField)s,
/// [`Compartment`] instances, priority wrappers ([`Extension::with_priority`]), or nested
/// groups (`Extension::from([ext1, ext2])`).
///
/// If the exact same `Extension` instance is included multiple times in a configuration tree,
/// it is automatically deduplicated during flattening.
///
/// # Examples
///
/// ```rust
/// use texera_core::{AllowMultipleSelections, Extension, ReadOnly, State, TabSize};
///
/// let editor_config = Extension::from([
///     TabSize(2).into(),
///     ReadOnly(false).into(),
///     AllowMultipleSelections(true).into(),
/// ]);
///
/// let state = State::from("code").with_extensions(editor_config);
/// assert_eq!(state.tab_size(), 2);
/// ```
#[derive(Clone, Debug)]
pub enum Extension {
    /// A nested group of extensions.
    Group(Vec<Extension>),
    /// An extension wrapped with an explicit [`Priority`] override.
    Prec(Priority, Box<Extension>),
    /// A static or dynamic facet value provider.
    FacetProvider(Arc<ErasedFacetProvider>),
    /// A custom [`StateField`](crate::StateField) registration.
    StateField(ErasedStateField),
    /// A named, dynamically reconfigurable [`Compartment`] slot.
    Compartment(usize, Box<Extension>),
}

impl Default for Extension {
    fn default() -> Self {
        Self::Group(Vec::new())
    }
}

impl PartialEq for Extension {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Group(a), Self::Group(b)) => a == b,
            (Self::Prec(pa, a), Self::Prec(pb, b)) => pa == pb && a == b,
            (Self::FacetProvider(a), Self::FacetProvider(b)) => a.id == b.id,
            (Self::StateField(a), Self::StateField(b)) => a.type_id == b.type_id,
            (Self::Compartment(ia, a), Self::Compartment(ib, b)) => ia == ib && a == b,
            _ => false,
        }
    }
}

impl Eq for Extension {}

/// A dynamic configuration container that allows replacing a sub-tree of [`Extension`]s
/// on the fly via a transaction [`StateEffect`].
///
/// Each `Compartment` instance may appear at most once in a state's configuration tree;
/// attempting to include the same compartment twice returns a
/// [`DuplicateCompartment`](crate::ErrorTypes::DuplicateCompartment) error.
///
/// # Examples
///
/// ```rust
/// use texera_core::{Compartment, State, TabSize, tr};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let indent = Compartment::new();
/// let state = State::default().with_extensions(indent.of(TabSize(2)));
/// assert_eq!(state.tab_size(), 2);
///
/// // Reconfigure only the `indent` compartment via a transaction effect
/// let state = state.update(tr!({
///     effects: [indent.reconfigure(TabSize(8))],
/// }))?.into_state();
/// assert_eq!(state.tab_size(), 8);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Compartment {
    pub(crate) id: usize,
}

impl Default for Compartment {
    fn default() -> Self {
        Self::new()
    }
}

impl Compartment {
    /// Creates a new, unique [`Compartment`] handle.
    pub fn new() -> Self {
        Self {
            id: super::utils::next_id(),
        }
    }

    /// Binds an initial [`Extension`] to this compartment for inclusion in a [`State`] configuration.
    pub fn of(&self, ext: impl Into<Extension>) -> Extension {
        Extension::Compartment(self.id, Box::new(ext.into()))
    }

    /// Creates a [`StateEffect`] that replaces the active [`Extension`] inside this compartment
    /// when dispatched in a transaction.
    pub fn reconfigure(&self, ext: impl Into<Extension>) -> StateEffect {
        StateEffect::new(CompartmentEffect {
            compartment_id: self.id,
            extension: ext.into(),
        })
    }

    /// Returns a reference to the [`Extension`] currently active in this compartment for `state`,
    /// or `None` if this compartment is not registered in `state`.
    pub fn get<'a>(&self, state: &'a State) -> Option<&'a Extension> {
        state.config.compartments.get(&self.id)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct CompartmentEffect {
    pub(crate) compartment_id: usize,
    pub(crate) extension: Extension,
}
impl Effect for CompartmentEffect {}

#[derive(Clone, Debug)]
pub(crate) struct ReconfigureEffect(pub(crate) Extension);
impl Effect for ReconfigureEffect {}

#[derive(Clone, Debug)]
pub(crate) struct AppendConfigEffect(pub(crate) Extension);
impl Effect for AppendConfigEffect {}
