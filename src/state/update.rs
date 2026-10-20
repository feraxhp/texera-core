use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;

use mynox::Error;

use crate::changes::iterator::ChangeIterator;
use crate::changes::{Composable, Mapable};
use crate::extension::{AppendConfigEffect, CompartmentEffect, ReconfigureEffect};
use crate::state::{
    AllowMultipleSelections, ChangeFilter, ChangeFilterResult, TransactionExtender,
    TransactionFilter, TransactionResult,
};
use crate::{Assoc, ChangeSet, ChangeSpec, Extension, Selection, SelectionRange, Text, Transaction, TransactionSpec};

use super::configuration::SlotAddr;


/// Trait implemented by types that can be passed to [`State::update`](crate::State::update):
/// a single [`TransactionSpec`], a `Vec<TransactionSpec>`, a fixed-size array `[TransactionSpec; N]`,
/// or a slice `&[TransactionSpec]`.
pub trait IntoTransactionSpecs {
    /// Converts `self` into a vector of [`TransactionSpec`]s to be combined and applied.
    fn into_specs(self) -> Vec<TransactionSpec>;
}

impl IntoTransactionSpecs for TransactionSpec {
    fn into_specs(self) -> Vec<TransactionSpec> {
        vec![self]
    }
}

impl IntoTransactionSpecs for Vec<TransactionSpec> {
    fn into_specs(self) -> Vec<TransactionSpec> {
        self
    }
}

impl<const N: usize> IntoTransactionSpecs for [TransactionSpec; N] {
    fn into_specs(self) -> Vec<TransactionSpec> {
        self.into_iter().collect()
    }
}

impl IntoTransactionSpecs for &[TransactionSpec] {
    fn into_specs(self) -> Vec<TransactionSpec> {
        self.to_vec()
    }
}


impl super::State {
    /// Resolves and applies one or more [`TransactionSpec`]s to `self`, returning a
    /// [`TransactionResult`] with the new [`State`](crate::State) and transaction metadata.
    ///
    /// # Execution Pipeline
    ///
    /// 1. **[`TransactionFilter`]**: Rewrites or replaces specs (unless `filter: false` is set).
    /// 2. **Spec Combination**: Merges concurrent specs (mapping each against prior changes) or
    ///    sequential specs (`sequential: true`, composed directly).
    /// 3. **[`ChangeFilter`]**: Cancels or splits document changes over protected ranges.
    /// 4. **[`TransactionExtender`]**: Runs extenders in reverse registration order to attach extra
    ///    annotations or effects.
    /// 5. **Reconfiguration**: Applies any [`Compartment::reconfigure`](crate::Compartment::reconfigure),
    ///    [`StateEffect::append_config`](crate::StateEffect::append_config), or
    ///    [`StateEffect::reconfigure`](crate::StateEffect::reconfigure) effects while preserving
    ///    existing [`StateField`](crate::StateField) values.
    /// 6. **Document & Selection Application**: Applies the final [`ChangeSet`] to the document,
    ///    maps or clamps the [`Selection`], and enforces [`AllowMultipleSelections`].
    /// 7. **State Fields & Dynamic Facets**: Updates all [`StateField`](crate::StateField)s and
    ///    incrementally recomputes only those dynamic [`Facet`](crate::Facet)s whose dependencies changed.
    ///
    /// # Examples
    ///
    /// ```
    /// use texera_core::{State, tr};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let state = State::from("hello world");
    ///
    /// // Apply two concurrent specs in a single update call (both reference original coordinates):
    /// let res = state.update([
    ///     tr!({ changes: { from: 0, to: 5, insert: "hi" } }),
    ///     tr!({ changes: { from: 6, to: 11, insert: "rust" } }),
    /// ])?;
    ///
    /// assert_eq!(res.state_ref().doc().to_string(), "hi rust");
    /// # Ok(())
    /// # }
    /// ```
    pub fn update(mut self, tr: impl IntoTransactionSpecs) -> Result<TransactionResult, Error> {
        let start_state = self.clone();
        let mut specs = tr.into_specs();
        let filter_enabled = specs.iter().all(|s| s.filter.unwrap_or(true));

        // 1. Resolver TransactionFilters si el filtrado está activo
        if filter_enabled {
            let tx_filters = self.facet::<TransactionFilter>();
            if !tx_filters.is_empty() {
                for tf in tx_filters {
                    let mut next_specs = Vec::new();
                    for s in specs {
                        let preview_tx = self.build_preview_tx(&s)?;
                        next_specs.extend((tf.0)(&preview_tx)?);
                    }
                    specs = next_specs;
                }
            }
        }

        let mut tx = self.combine_specs(specs)?;

        // 2. Aplicar ChangeFilters si el filtrado está activo y hay cambios
        if filter_enabled && tx.changes.is_some() {
            let change_filters = self.facet::<ChangeFilter>();
            if !change_filters.is_empty() {
                let mut cancelled = false;
                let mut excluded_ranges: Vec<(usize, usize)> = Vec::new();

                for cf in change_filters {
                    match (cf.0)(&tx) {
                        ChangeFilterResult::Bool(true) => {}
                        ChangeFilterResult::Bool(false) => {
                            cancelled = true;
                            break;
                        }
                        ChangeFilterResult::Ranges(coords) => {
                            for pair in coords.chunks(2) {
                                if pair.len() == 2 && pair[0] < pair[1] {
                                    excluded_ranges.push((pair[0], pair[1]));
                                }
                            }
                        }
                    }
                }

                if cancelled {
                    tx.changes = None;
                } else if !excluded_ranges.is_empty() {
                    if let Some(ch) = &tx.changes {
                        let filtered = Self::filter_changeset(ch, &excluded_ranges, self.doc.len_chars())?;
                        tx.changes = Some(filtered);
                    }
                }
            }
        }

        // 3. Ejecutar TransactionExtenders (en orden inverso al registro, como CodeMirror 6)
        let extenders = self.facet::<TransactionExtender>();
        if !extenders.is_empty() {
            tx.doc = match &tx.changes {
                Some(ch) => ch.apply(self.doc.clone())?,
                None => self.doc.clone(),
            };
            for ext in extenders.iter().rev() {
                let extra = (ext.0)(&tx);
                if let Some(ue) = extra.user_event {
                    tx.annotations.push(crate::Annotation::user_event(ue));
                }
                tx.annotations.extend(extra.annotations);
                tx.effects.extend(extra.effects);
            }
        }

        // 4. Camino 6B: Reconfiguración sobre el estado previo (intermediateState) si hay efectos de configuración
        let has_reconfig = tx.effects.iter().any(|eff| {
            eff.is::<ReconfigureEffect>()
                || eff.is::<AppendConfigEffect>()
                || eff.is::<CompartmentEffect>()
        });

        if has_reconfig {
            let mut base = self.config.base.clone();
            let mut overrides = self.config.compartments.clone();
            for eff in &tx.effects {
                if let Some(ReconfigureEffect(new_base)) = eff.get::<ReconfigureEffect>() {
                    base = new_base.clone();
                } else if let Some(AppendConfigEffect(ext)) = eff.get::<AppendConfigEffect>() {
                    base = Extension::Group(vec![base, ext.clone()]);
                } else if let Some(comp_eff) = eff.get::<CompartmentEffect>() {
                    overrides.insert(comp_eff.compartment_id, comp_eff.extension.clone());
                }
            }
            self = self.reconfigure_with(base, Some(&overrides), Some(&start_state))?;
        }

        // 5. Aplicar cambios al documento y selección
        let mut recorded_changes = Vec::new();
        let mut recorded_selections = Vec::new();

        if let Some(changes) = &tx.changes {
            self.doc = changes.apply(self.doc)?;
            recorded_changes.push(changes.clone());
        }
        tx.doc = self.doc.clone();

        let doc_len = self.doc.len_chars();
        let mut new_selection = match (&tx.selection, &tx.changes) {
            (Some(sel), _) => {
                let clamped = Self::clamp_selection(sel, doc_len);
                recorded_selections.push(clamped.clone());
                clamped
            }
            (None, Some(changes)) => self.selection.map(&changes.description(), Assoc::After)?,
            (None, None) => self.selection.clone(),
        };

        if !self.facet::<AllowMultipleSelections>() && new_selection.count() > 1 {
            new_selection = new_selection.as_single();
        }
        self.selection = new_selection;
        self.raw_selection = None;

        // 6. Camino 6A: Actualizar StateFields y Facetas Dinámicas
        let next_state = self.apply(&tx);

        Ok(TransactionResult {
            state: next_state,
            start_state,
            changes: recorded_changes,
            selection: recorded_selections,
            annotations: tx.annotations,
            effects: tx.effects,
            scroll_into_view: tx.scroll_into_view,
        })
    }

    fn build_preview_tx(&self, spec: &TransactionSpec) -> Result<Transaction, Error> {
        let mut tx = Transaction::try_from((self.doc.len_chars(), spec.clone()))?;
        tx.start_state = self.clone();
        tx.doc = match &tx.changes {
            Some(ch) => ch.apply(self.doc.clone())?,
            None => self.doc.clone(),
        };
        Ok(tx)
    }

    fn combine_specs(&self, specs: Vec<TransactionSpec>) -> Result<Transaction, Error> {
        let base_len = self.doc.len_chars();
        let mut total_changes: Option<ChangeSet> = None;
        let mut final_selection: Option<Selection> = None;
        let mut annotations = Vec::new();
        let mut effects: Vec<crate::StateEffect> = Vec::new();
        let mut scroll_into_view = false;
        let last_spec = specs.last().cloned().unwrap_or_default();

        for spec in specs {
            let is_seq = spec.sequential.unwrap_or(false);
            let mut map_for_current: Option<crate::ChangeDesc> = None;

            if !spec.changes.is_empty() {
                let target_len = if is_seq {
                    total_changes.as_ref().map(|c| c.len_new()).unwrap_or(base_len)
                } else {
                    base_len
                };
                let cs = ChangeSet::from_slice(&spec.changes, target_len)?;
                let effective_cs = match total_changes.take() {
                    None => {
                        total_changes = Some(cs.clone());
                        cs
                    }
                    Some(prev) => {
                        if is_seq {
                            total_changes = Some(prev.compose(&cs)?);
                            cs
                        } else {
                            let mapped = cs.map(&prev, false)?;
                            let prev_mapped = prev.map(&cs, true)?;
                            map_for_current = Some(prev_mapped.description().clone());
                            total_changes = Some(prev.compose(&mapped)?);
                            mapped
                        }
                    }
                };
                let eff_desc = effective_cs.description();
                if is_seq {
                    if let Some(sel) = final_selection.take() {
                        final_selection = Some(sel.map(eff_desc, Assoc::After)?);
                    }
                }
                if !effects.is_empty() {
                    effects = effects.into_iter().filter_map(|e| e.map(eff_desc)).collect();
                }
            }

            if let Some(sel) = spec.selection {
                final_selection = Some(match &map_for_current {
                    Some(desc) => sel.map(desc, Assoc::After)?,
                    None => sel,
                });
            }
            if let Some(ue) = spec.user_event {
                annotations.push(crate::Annotation::user_event(ue));
            }
            annotations.extend(spec.annotations);
            if let Some(desc) = &map_for_current {
                effects.extend(spec.effects.into_iter().filter_map(|e| e.map(desc)));
            } else {
                effects.extend(spec.effects);
            }
            if spec.scroll_into_view.unwrap_or(false) {
                scroll_into_view = true;
            }
        }

        let preview_doc = match &total_changes {
            Some(ch) => ch.apply(self.doc.clone())?,
            None => self.doc.clone(),
        };

        Ok(Transaction {
            changes: total_changes,
            selection: final_selection,
            annotations,
            effects,
            scroll_into_view,
            doc: preview_doc,
            start_state: self.clone(),
            spec: last_spec,
        })
    }

    fn filter_changeset(
        cs: &ChangeSet,
        excluded: &[(usize, usize)],
        doc_len: usize,
    ) -> Result<ChangeSet, Error> {
        let mut allowed_specs = Vec::new();

        for ch in cs.iter_changes(false) {
            let from = ch.map.old.from;
            let to = ch.map.old.to;
            let text = ch.text;

            if from == to {
                let is_excluded = excluded.iter().any(|&(ef, et)| ef < from && from < et);
                if !is_excluded {
                    allowed_specs.push(ChangeSpec::new(from, Some(to), Some(text))?);
                }
            } else {
                let mut segments = vec![(from, to)];
                for &(ef, et) in excluded {
                    let mut next_segments = Vec::new();
                    for (sf, st) in segments {
                        if et <= sf || ef >= st {
                            next_segments.push((sf, st));
                        } else {
                            if sf < ef {
                                next_segments.push((sf, ef));
                            }
                            if et < st {
                                next_segments.push((et, st));
                            }
                        }
                    }
                    segments = next_segments;
                }
                for (i, (sf, st)) in segments.into_iter().enumerate() {
                    let ins = if i == 0 && text.len_chars() > 0 {
                        Some(text.clone())
                    } else {
                        None
                    };
                    allowed_specs.push(ChangeSpec::new(sf, Some(st), ins)?);
                }
            }
        }

        ChangeSet::from_slice(&allowed_specs, doc_len)
    }

    fn clamp_selection(sel: &Selection, max_len: usize) -> Selection {
        sel.clone()
            .into_iter()
            .map(|(r, is_main)| {
                let anchor = r.anchor().min(max_len);
                let head = r.head().min(max_len);
                let clamped = if anchor == head {
                    SelectionRange::cursor(head)
                } else {
                    SelectionRange::range(anchor, head)
                }
                ;
                (clamped, is_main)
            })
            .collect()
    }
    
    pub(crate) fn apply(self, tx: &Transaction) -> Self {
        let mut next_state = Self {
            doc: self.doc().clone(),
            selection: self.selection().clone(),
            raw_selection: None,
            config: Arc::clone(&self.config),
            dynamic_slots: Vec::with_capacity(self.dynamic_slots.len()),
            provider_cache: HashMap::with_capacity(self.provider_cache.len()),
        };

        // 1. Actualizar StateFields
        for (idx, field) in self.config.dynamic_fields.iter().enumerate() {
            let updated = (field.update_fn)(&self.dynamic_slots[idx], tx);
            next_state.dynamic_slots.push(updated);
        }

        // 2. Actualizar Facetas Dinámicas sólo si alguna dependencia cambió
        let base = self.config.dynamic_fields.len();
        let doc_type = TypeId::of::<Text>();
        let sel_type = TypeId::of::<Selection>();

        let dep_changed_in_tx = |dep_id: TypeId, old_slots: &[Arc<crate::AnySync>], new_slots: &[Arc<crate::AnySync>]| -> bool {
            if dep_id == doc_type {
                tx.doc_changed()
            } else if dep_id == sel_type {
                tx.selection_changed()
            } else {
                matches!(
                    self.config.address.get(&dep_id),
                    Some(&SlotAddr::Dynamic(d))
                        if !Arc::ptr_eq(&old_slots[d as usize], &new_slots[d as usize])
                )
            }
        };

        for (i, group) in self.config.dynamic_facets.iter().enumerate() {
            let slot_idx = base + i;
            let any_dep_changed = group.iter().flat_map(|p| p.deps.iter()).any(|&dep_id| {
                dep_changed_in_tx(dep_id, &self.dynamic_slots, &next_state.dynamic_slots)
            });

            let val = if any_dep_changed {
                let mut raw_vals = Vec::with_capacity(group.len());
                let mut any_input_changed = false;

                for p in group {
                    if p.deps.is_empty() {
                        raw_vals.push((p.eval)(&next_state));
                    } else {
                        let p_changed = p.deps.iter().any(|&dep_id| {
                            dep_changed_in_tx(dep_id, &self.dynamic_slots, &next_state.dynamic_slots)
                        });
                        if p_changed || !self.provider_cache.contains_key(&p.id) {
                            let new_in = (p.eval)(&next_state);
                            if let Some(old_in) = self.provider_cache.get(&p.id) {
                                if !(p.compare_input)(old_in, &new_in) {
                                    any_input_changed = true;
                                }
                            } else {
                                any_input_changed = true;
                            }
                            next_state.provider_cache.insert(p.id, Arc::clone(&new_in));
                            raw_vals.push(new_in);
                        } else {
                            let cached = Arc::clone(self.provider_cache.get(&p.id).unwrap());
                            next_state.provider_cache.insert(p.id, Arc::clone(&cached));
                            raw_vals.push(cached);
                        }
                    }
                }

                if !any_input_changed {
                    Arc::clone(&self.dynamic_slots[slot_idx])
                } else {
                    let new_val = (group[0].compute_static)(&raw_vals);
                    let old_val = &self.dynamic_slots[slot_idx];
                    if (group[0].compare_output)(old_val, &new_val) {
                        Arc::clone(old_val)
                    } else {
                        new_val
                    }
                }
            } else {
                for p in group {
                    if !p.deps.is_empty()
                        && let Some(cached) = self.provider_cache.get(&p.id)
                    {
                        next_state.provider_cache.insert(p.id, Arc::clone(cached));
                    }
                }
                Arc::clone(&self.dynamic_slots[slot_idx])
            };
            next_state.dynamic_slots.push(val);
        }

        next_state
    }
}
