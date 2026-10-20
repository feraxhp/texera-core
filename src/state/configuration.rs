use std::collections::HashMap;
use std::any::TypeId;
use std::sync::Arc;


use crate::ErasedFacetProvider;
use crate::ErasedStateField;
use crate::Extension;
use crate::AnySync;

/// Internal slot address distinguishing static facet outputs from dynamic slots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotAddr {
    /// Index into `Configuration::static_values` (evaluated once at configuration time).
    Static(u32),
    /// Index into `State::dynamic_slots` (re-evaluated on transactions when dependencies change).
    Dynamic(u32),
}

/// Compiled extension configuration shared across [`State`](crate::State) instances via `Arc`.
///
/// Stores the flattened extension tree, compartment map, slot address table, static facet values,
/// [`StateField`](crate::StateField) descriptors, and topologically sorted dynamic facet providers.
#[derive(Debug, Default)]
pub struct Configuration {
    pub(crate) base: Extension,
    pub(crate) compartments: HashMap<usize, Extension>,
    pub(crate) address: HashMap<TypeId, SlotAddr>,
    pub(crate) comparators: HashMap<TypeId, fn(&Arc<AnySync>, &Arc<AnySync>) -> bool>,
    pub(crate) static_values: Vec<Arc<AnySync>>,
    pub(crate) dynamic_fields: Vec<ErasedStateField>,
    pub(crate) dynamic_facets: Vec<Vec<Arc<ErasedFacetProvider>>>,
}
