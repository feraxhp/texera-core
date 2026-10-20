use std::any::{Any, TypeId};
use std::sync::Arc;

use crate::{AnySync, State, StateField};

use super::ErasedFacetProvider;
use super::utils::{dep, next_id};
use super::IntoExtension;
use super::Extension;


/// Low-level trait for defining a facet with custom `Input` and `Output` types and a custom
/// `combine` reducer.
///
/// For simple single-value (`T` with `Default`) or multi-value (`Vec<T>`) facets where
/// `Input == Self`, implement the higher-level [`Facet`] trait instead (which automatically
/// implements `FacetDef`).
///
/// # Examples
///
/// ```rust
/// use texera_core::{Extension, FacetDef, State, Text, dep};
///
/// struct MaxLimit;
/// impl FacetDef for MaxLimit {
///     type Input = usize;
///     type Output = Option<usize>;
///
///     fn combine(inputs: &[usize]) -> Option<usize> {
///         inputs.iter().copied().max()
///     }
///     fn compare_input(a: &usize, b: &usize) -> bool { a == b }
///     fn compare(a: &Option<usize>, b: &Option<usize>) -> bool { a == b }
/// }
///
/// let state = State::from("hello").with_extensions(Extension::from([
///     Extension::facet::<MaxLimit>(10),
///     MaxLimit::compute([dep::<Text>()], |s| s.doc().len_chars() * 10),
/// ]));
/// assert_eq!(state.facet::<MaxLimit>(), Some(50));
/// ```
pub trait FacetDef: Any + Send + Sync + 'static {
    /// Type contributed by each individual provider of this facet.
    type Input: Clone + Send + Sync + 'static;
    /// Type returned when querying [`State::facet::<Self>()`](crate::State::facet).
    type Output: Clone + Send + Sync + 'static;

    /// Combines all active inputs (ordered by [`Priority`](crate::Priority)) into the facet's output.
    fn combine(inputs: &[Self::Input]) -> Self::Output;

    /// Compares two provider inputs to short-circuit recomputation when dynamic inputs haven't changed.
    fn compare_input(_a: &Self::Input, _b: &Self::Input) -> bool {
        false
    }

    /// Compares two combined outputs to avoid invalidating downstream facets when the combined output is unchanged.
    fn compare(_a: &Self::Output, _b: &Self::Output) -> bool {
        false
    }

    /// Creates a dynamic facet provider whose value is computed from `state` whenever any
    /// dependency in `deps` (created via [`dep`](crate::dep)) changes.
    fn compute<const N: usize>(
        deps: [TypeId; N],
        eval: impl Fn(&State) -> Self::Input + Send + Sync + 'static,
    ) -> Extension
    where
        Self: Sized,
    {
        Extension::compute::<Self, N>(deps, eval)
    }

    /// Creates a dynamic facet provider that contributes zero or more values (`Vec<Self::Input>`)
    /// computed from `state` whenever any dependency in `deps` changes.
    fn compute_multi<const N: usize>(
        deps: [TypeId; N],
        eval: impl Fn(&State) -> Vec<Self::Input> + Send + Sync + 'static,
    ) -> Extension
    where
        Self: Sized,
    {
        Extension::compute_multi::<Self, N>(deps, eval)
    }

    /// Creates a dynamic facet provider that reads its input directly from a [`StateField`] `S`
    /// whose `S::Value == Self::Input`.
    fn from_field<S: StateField<Value = Self::Input>>() -> Extension
    where
        Self: Sized,
    {
        Self::compute([dep::<S>()], |state| {
            state
                .field::<S>()
                .expect("StateField referenced by Facet::from_field is not present in State")
                .clone()
        })
    }

    /// Creates a dynamic facet provider derived from a [`StateField`] `S` via a mapping function `get`.
    fn from_field_with<S: StateField>(
        get: impl Fn(&S::Value) -> Self::Input + Send + Sync + 'static,
    ) -> Extension
    where
        Self: Sized,
    {
        Self::compute([dep::<S>()], move |state| {
            let val = state
                .field::<S>()
                .expect("StateField referenced by Facet::from_field_with is not present in State");
            get(val)
        })
    }

    /// Creates a dynamic multi-value facet provider derived from a [`StateField`] `S` via `get`.
    fn from_field_multi<S: StateField>(
        get: impl Fn(&S::Value) -> Vec<Self::Input> + Send + Sync + 'static,
    ) -> Extension
    where
        Self: Sized,
    {
        Self::compute_multi([dep::<S>()], move |state| {
            let val = state
                .field::<S>()
                .expect("StateField referenced by Facet::from_field_multi is not present in State");
            get(val)
        })
    }
}

/// Marker mode for [`Facet`]s that resolve to a single value (taking the highest-priority input,
/// or `T::default()` if no provider is registered).
pub struct Single;

/// Marker mode for [`Facet`]s that collect all registered inputs into a `Vec<T>` ordered by priority.
pub struct Multi;

/// Strategy trait connecting [`Single`] and [`Multi`] modes to [`Facet`].
pub trait FacetMode<T> {
    /// Combined output type produced by this mode (`T` for [`Single`], `Vec<T>` for [`Multi`]).
    type Output: Clone + Send + Sync + 'static;
    /// Combines the slice of inputs according to this mode.
    fn combine(inputs: &[T]) -> Self::Output;
    /// Compares two combined outputs using `compare_input`.
    fn compare(a: &Self::Output, b: &Self::Output, compare_input: fn(&T, &T) -> bool) -> bool;
}

impl<T: Clone + Default + Send + Sync + 'static> FacetMode<T> for Single {
    type Output = T;
    fn combine(inputs: &[T]) -> T {
        inputs.first().cloned().unwrap_or_default()
    }
    fn compare(a: &T, b: &T, compare_input: fn(&T, &T) -> bool) -> bool {
        compare_input(a, b)
    }
}

impl<T: Clone + Send + Sync + 'static> FacetMode<T> for Multi {
    type Output = Vec<T>;
    fn combine(inputs: &[T]) -> Vec<T> {
        inputs.to_vec()
    }
    fn compare(a: &Vec<T>, b: &Vec<T>, compare_input: fn(&T, &T) -> bool) -> bool {
        a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| compare_input(x, y))
    }
}

/// Ergonomic trait for defining single-value ([`Single`]) or multi-value ([`Multi`]) facets.
///
/// Implementing `Facet` automatically implements [`FacetDef`], [`IntoExtension`], and
/// `From<Self> for Extension`.
///
/// # Examples
///
/// ```rust
/// use texera_core::{Extension, Facet, Multi, Single, State};
///
/// #[derive(Clone, Debug, Default, PartialEq, Eq)]
/// struct Theme(&'static str);
/// impl Facet for Theme {
///     type Mode = Single;
///     fn compare_input(a: &Self, b: &Self) -> bool { a == b }
/// }
///
/// #[derive(Clone, Debug, PartialEq, Eq)]
/// struct Plugin(&'static str);
/// impl Facet for Plugin {
///     type Mode = Multi;
///     fn compare_input(a: &Self, b: &Self) -> bool { a == b }
/// }
///
/// let state = State::from(Extension::from([
///     Theme("gruvbox").into(),
///     Theme("dracula").into(),
///     Plugin("lsp").into(),
///     Plugin("git").into(),
/// ]));
///
/// assert_eq!(state.facet::<Theme>(), Theme("gruvbox"));
/// assert_eq!(state.facet::<Plugin>(), vec![Plugin("lsp"), Plugin("git")]);
/// ```
pub trait Facet: Clone + Send + Sync + 'static {
    /// Either [`Single`] (output is `Self`) or [`Multi`] (output is `Vec<Self>`).
    type Mode: FacetMode<Self>;

    /// Optional equality check used to avoid recomputing downstream dynamic facets when unchanged.
    fn compare_input(_a: &Self, _b: &Self) -> bool {
        false
    }
}

impl<T: Facet> FacetDef for T {
    type Input = T;
    type Output = <T::Mode as FacetMode<T>>::Output;

    fn combine(inputs: &[T]) -> Self::Output {
        <T::Mode as FacetMode<T>>::combine(inputs)
    }

    fn compare_input(a: &T, b: &T) -> bool {
        <T as Facet>::compare_input(a, b)
    }

    fn compare(a: &Self::Output, b: &Self::Output) -> bool {
        <T::Mode as FacetMode<T>>::compare(a, b, <T as Facet>::compare_input)
    }
}

impl<F: FacetDef<Input = F>> IntoExtension for F {
    fn into_ext(self) -> Extension {
        Extension::facet::<F>(self)
    }
}

impl Extension {
    /// Creates a static facet provider [`Extension`] supplying `value` to facet `F`.
    pub fn facet<F: FacetDef>(value: F::Input) -> Extension {
        let val: Arc<AnySync> = Arc::new(value);
        Extension::FacetProvider(Arc::new(ErasedFacetProvider {
            id: next_id(),
            facet_type_id: TypeId::of::<F>(),
            deps: Box::new([]),
            eval: Box::new(move |_| Arc::clone(&val)),
            compute_static: Self::combine_erased::<F>,
            compare_input: Self::compare_input_erased::<F>,
            compare_output: Self::compare_output_erased::<F>,
        }))
    }
    
    fn combine_erased<F: FacetDef>(raw_inputs: &[Arc<AnySync>]) -> Arc<AnySync> {
        let mut typed: Vec<F::Input> = Vec::with_capacity(raw_inputs.len());
        for any in raw_inputs {
            if let Some(single) = any.downcast_ref::<F::Input>() {
                typed.push(single.clone());
            } else if let Some(multi) = any.downcast_ref::<Vec<F::Input>>() {
                typed.extend(multi.iter().cloned());
            }
        }
        Arc::new(F::combine(&typed))
    }

    fn compare_input_erased<F: FacetDef>(a: &Arc<AnySync>, b: &Arc<AnySync>) -> bool {
        if let (Some(a_val), Some(b_val)) = (a.downcast_ref::<F::Input>(), b.downcast_ref::<F::Input>()) {
            F::compare_input(a_val, b_val)
        } else if let (Some(a_vec), Some(b_vec)) = (a.downcast_ref::<Vec<F::Input>>(), b.downcast_ref::<Vec<F::Input>>()) {
            a_vec.len() == b_vec.len() && a_vec.iter().zip(b_vec.iter()).all(|(x, y)| F::compare_input(x, y))
        } else {
            false
        }
    }

    fn compare_output_erased<F: FacetDef>(a: &Arc<AnySync>, b: &Arc<AnySync>) -> bool {
        match (a.downcast_ref::<F::Output>(), b.downcast_ref::<F::Output>()) {
            (Some(a_val), Some(b_val)) => F::compare(a_val, b_val),
            _ => false,
        }
    }
    
    /// Creates a dynamic facet provider [`Extension`] for facet `F` with dependencies `deps`.
    pub fn compute<F: FacetDef, const N: usize>(
        deps: [TypeId; N],
        eval: impl Fn(&State) -> F::Input + Send + Sync + 'static,
    ) -> Extension {
        Extension::FacetProvider(Arc::new(ErasedFacetProvider {
            id: next_id(),
            facet_type_id: TypeId::of::<F>(),
            deps: Box::new(deps),
            eval: Box::new(move |state| Arc::new(eval(state))),
            compute_static: Self::combine_erased::<F>,
            compare_input: Self::compare_input_erased::<F>,
            compare_output: Self::compare_output_erased::<F>,
        }))
    }

    /// Creates a dynamic multi-value facet provider [`Extension`] for facet `F` with dependencies `deps`.
    pub fn compute_multi<F: FacetDef, const N: usize>(
        deps: [TypeId; N],
        eval: impl Fn(&State) -> Vec<F::Input> + Send + Sync + 'static,
    ) -> Extension {
        Extension::FacetProvider(Arc::new(ErasedFacetProvider {
            id: next_id(),
            facet_type_id: TypeId::of::<F>(),
            deps: Box::new(deps),
            eval: Box::new(move |state| Arc::new(eval(state))),
            compute_static: Self::combine_erased::<F>,
            compare_input: Self::compare_input_erased::<F>,
            compare_output: Self::compare_output_erased::<F>,
        }))
    }
}
