use std::{any::TypeId, collections::{HashMap, HashSet}, sync::Arc};

use mynox::Error;

use crate::state::configuration::Configuration;
use crate::state::configuration::SlotAddr;
use crate::state::AllowMultipleSelections;
use crate::ErasedFacetProvider;
use crate::Selection;
use crate::Extension;
use crate::AnySync;
use crate::Text;

use super::State;


impl State {
    /// Attaches one or more [`Extension`]s (facets, state fields, compartments, priorities,
    /// or groups of extensions) to this [`State`], compiling the configuration and
    /// initializing all fields and facets.
    ///
    /// # Panics
    ///
    /// Panics if the same [`Compartment`](crate::Compartment) instance is mounted more than once
    /// in the extension tree, or if dynamic facets form a cyclic dependency. Use
    /// [`State::try_with_extensions`] for a non-panicking `Result` on duplicate compartments.
    ///
    /// # Examples
    ///
    /// ```
    /// use texera_core::{State, Extension, TabSize, ReadOnly, AllowMultipleSelections};
    ///
    /// let state = State::from("fn main() {}")
    ///     .with_extensions(Extension::from([
    ///         TabSize(2).into(),
    ///         ReadOnly(false).into(),
    ///         AllowMultipleSelections(true).into(),
    ///     ]));
    ///
    /// assert_eq!(state.tab_size(), 2);
    /// assert!(!state.read_only());
    /// ```
    pub fn with_extensions(self, extension: impl Into<Extension>) -> Self {
        self.try_with_extensions(extension)
            .expect("Duplicate use of compartment")
    }

    /// Fallible version of [`State::with_extensions`] that returns an error if a
    /// [`Compartment`](crate::Compartment) is registered more than once in `extension`.
    ///
    /// # Examples
    ///
    /// ```
    /// use texera_core::{State, Compartment, Extension, TabSize};
    ///
    /// let comp = Compartment::new();
    /// let duplicate = Extension::from([comp.of(TabSize(2)), comp.of(TabSize(4))]);
    ///
    /// assert!(State::from("text").try_with_extensions(duplicate).is_err());
    /// ```
    pub fn try_with_extensions(self, extension: impl Into<Extension>) -> Result<Self, Error> {
        self.reconfigure_with(extension.into(), None, None)
    }

    pub(crate) fn reconfigure_with(
        self,
        base: Extension,
        compartment_overrides: Option<&HashMap<usize, Extension>>,
        old_state: Option<&State>,
    ) -> Result<Self, Error> {
        let (providers, fields, compartments) =
            Extension::flatten(base.clone(), compartment_overrides)?;

        let mut address = HashMap::new();
        let mut comparators: HashMap<TypeId, fn(&Arc<AnySync>, &Arc<AnySync>) -> bool> =
            HashMap::new();
        let mut dynamic_fields = Vec::with_capacity(fields.len());

        // 1. Registrar StateFields en los primeros slots dinámicos
        for field in fields {
            let idx = dynamic_fields.len() as u32;
            address.insert(field.type_id, SlotAddr::Dynamic(idx));
            dynamic_fields.push(field);
        }

        // 2. Agrupar proveedores de facetas por su TypeId (respetando orden de prioridad)
        let mut by_facet: HashMap<TypeId, Vec<Arc<ErasedFacetProvider>>> = HashMap::new();
        let mut facet_order = Vec::new();
        for p in providers {
            by_facet
                .entry(p.facet_type_id)
                .or_insert_with(|| {
                    facet_order.push(p.facet_type_id);
                    Vec::new()
                })
                .push(p);
        }

        let mut static_groups = Vec::new();
        let mut dynamic_entries: Vec<(TypeId, Vec<Arc<ErasedFacetProvider>>)> = Vec::new();

        for facet_id in facet_order {
            let group = by_facet.remove(&facet_id).unwrap();
            comparators.insert(facet_id, group[0].compare_output);
            let has_deps = group.iter().any(|p| !p.deps.is_empty());
            if has_deps {
                dynamic_entries.push((facet_id, group));
            } else {
                let idx = static_groups.len() as u32;
                address.insert(facet_id, SlotAddr::Static(idx));
                static_groups.push(group);
            }
        }

        let dynamic_facets =
            Self::topo_sort_dynamic_facets(dynamic_entries, &mut address, dynamic_fields.len());

        let mut state = Self {
            doc: self.doc,
            selection: self.selection,
            raw_selection: self.raw_selection,
            config: Arc::new(Configuration {
                base,
                compartments,
                address,
                comparators,
                static_values: Vec::new(),
                dynamic_fields,
                dynamic_facets,
            }),
            dynamic_slots: Vec::new(),
            provider_cache: HashMap::new(),
        };

        // 3. Evaluar facetas estáticas (sin dependencias)
        let static_values: Vec<Arc<AnySync>> = static_groups
            .iter()
            .map(|group| Self::eval_facet(group, &state))
            .collect();
        Arc::get_mut(&mut state.config).unwrap().static_values = static_values;

        // 4. Inicializar o preservar StateFields y luego Facetas Dinámicas
        for i in 0..state.config.dynamic_fields.len() {
            let field = &state.config.dynamic_fields[i];
            let val = if let Some(old) = old_state
                && let Some(&SlotAddr::Dynamic(old_idx)) = old.config.address.get(&field.type_id)
            {
                Arc::clone(&old.dynamic_slots[old_idx as usize])
            } else {
                (field.create_fn)(&state)
            };
            state.dynamic_slots.push(val);
        }

        let doc_type = TypeId::of::<Text>();
        let sel_type = TypeId::of::<Selection>();

        for i in 0..state.config.dynamic_facets.len() {
            let group = &state.config.dynamic_facets[i];
            let mut raw_vals = Vec::with_capacity(group.len());

            for p in group {
                if !p.deps.is_empty() {
                    if let Some(old) = old_state
                        && let Some(cached) = old.provider_cache.get(&p.id)
                    {
                        let dep_changed = p.deps.iter().any(|&dep_id| {
                            if dep_id == doc_type {
                                old.doc != state.doc
                            } else if dep_id == sel_type {
                                old.selection != state.selection
                            } else {
                                match (
                                    old.config.address.get(&dep_id),
                                    state.config.address.get(&dep_id),
                                ) {
                                    (None, None) => false,
                                    (Some(&old_addr), Some(&new_addr)) => {
                                        let old_val = old.get_slot(old_addr);
                                        let new_val = state.get_slot(new_addr);
                                        if Arc::ptr_eq(old_val, new_val) {
                                            false
                                        } else if let Some(&cmp) =
                                            state.config.comparators.get(&dep_id)
                                        {
                                            !cmp(old_val, new_val)
                                        } else {
                                            true
                                        }
                                    }
                                    _ => true,
                                }
                            }
                        });

                        if !dep_changed {
                            let val = Arc::clone(cached);
                            state.provider_cache.insert(p.id, Arc::clone(&val));
                            raw_vals.push(val);
                            continue;
                        }
                    }

                    let val = (p.eval)(&state);
                    state.provider_cache.insert(p.id, Arc::clone(&val));
                    raw_vals.push(val);
                } else {
                    raw_vals.push((p.eval)(&state));
                }
            }

            let combined = (group[0].compute_static)(&raw_vals);
            state.dynamic_slots.push(combined);
        }

        // 5. Normalizar selección múltiple según AllowMultipleSelections
        if state.facet::<AllowMultipleSelections>() {
            if let Some(raw) = state.raw_selection.take() {
                state.selection = raw;
            }
        } else if state.selection.count() > 1 {
            state.selection = state.selection.as_single();
        }

        Ok(state)
    }

    fn topo_sort_dynamic_facets(
        entries: Vec<(TypeId, Vec<Arc<ErasedFacetProvider>>)>,
        address: &mut HashMap<TypeId, SlotAddr>,
        base_offset: usize,
    ) -> Vec<Vec<Arc<ErasedFacetProvider>>> {
        let index_by_id: HashMap<TypeId, usize> = entries
            .iter()
            .enumerate()
            .map(|(i, (id, _))| (*id, i))
            .collect();

        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();
        let mut order = Vec::with_capacity(entries.len());

        fn dfs(
            idx: usize,
            entries: &[(TypeId, Vec<Arc<ErasedFacetProvider>>)],
            index_by_id: &HashMap<TypeId, usize>,
            visited: &mut HashSet<usize>,
            visiting: &mut HashSet<usize>,
            order: &mut Vec<usize>,
        ) {
            if visited.contains(&idx) {
                return;
            }
            if !visiting.insert(idx) {
                panic!("Cyclic dependency detected in dynamic facets");
            }
            for provider in &entries[idx].1 {
                for dep_id in provider.deps.iter() {
                    if let Some(&dep_idx) = index_by_id.get(dep_id) {
                        dfs(dep_idx, entries, index_by_id, visited, visiting, order);
                    }
                }
            }
            visiting.remove(&idx);
            visited.insert(idx);
            order.push(idx);
        }

        for i in 0..entries.len() {
            dfs(i, &entries, &index_by_id, &mut visited, &mut visiting, &mut order);
        }

        let mut entries_opt: Vec<Option<(TypeId, Vec<Arc<ErasedFacetProvider>>)>> =
            entries.into_iter().map(Some).collect();
        let mut sorted_groups = Vec::with_capacity(order.len());

        for idx in order {
            let (facet_id, group) = entries_opt[idx].take().unwrap();
            let slot_idx = (base_offset + sorted_groups.len()) as u32;
            address.insert(facet_id, SlotAddr::Dynamic(slot_idx));
            sorted_groups.push(group);
        }

        sorted_groups
    }
}
