use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use mynox::Error;

use crate::errors::duplicate_compartment;

use super::ErasedFacetProvider;
use super::ErasedStateField;
use super::Extension;
use super::Priority;


impl Extension {
    pub(crate) fn flatten(
        root: Extension,
        compartment_overrides: Option<&HashMap<usize, Extension>>,
    ) -> Result<(Vec<Arc<ErasedFacetProvider>>, Vec<ErasedStateField>, HashMap<usize, Extension>), Error> {
        enum FlatItem {
            Provider(Arc<ErasedFacetProvider>),
            Field(ErasedStateField),
        }
    
        let mut buckets: [Vec<FlatItem>; 5] = Default::default();
        let mut seen_provider_ids = HashSet::new();
        let mut seen_field_types = HashSet::new();
        let mut seen_compartments = HashSet::new();
        let mut compartments = HashMap::new();
    
        fn walk(
            ext: Extension,
            prio: Priority,
            buckets: &mut [Vec<FlatItem>; 5],
            seen_p: &mut HashSet<usize>,
            seen_f: &mut HashSet<TypeId>,
            seen_c: &mut HashSet<usize>,
            compartments: &mut HashMap<usize, Extension>,
            overrides: Option<&HashMap<usize, Extension>>,
        ) -> Result<(), Error> {
            match ext {
                Extension::Group(children) => {
                    for child in children {
                        walk(child, prio, buckets, seen_p, seen_f, seen_c, compartments, overrides)?;
                    }
                }
                Extension::Prec(new_prio, inner) => {
                    walk(*inner, new_prio, buckets, seen_p, seen_f, seen_c, compartments, overrides)?;
                }
                Extension::FacetProvider(provider) => {
                    if seen_p.insert(provider.id) {
                        buckets[prio as usize].push(FlatItem::Provider(provider));
                    }
                }
                Extension::StateField(field) => {
                    if seen_f.insert(field.type_id) {
                        let provided = field.provide.clone();
                        buckets[prio as usize].push(FlatItem::Field(field));
                        if let Some(ext) = provided {
                            walk(*ext, prio, buckets, seen_p, seen_f, seen_c, compartments, overrides)?;
                        }
                    }
                }
                Extension::Compartment(id, inner) => {
                    if !seen_c.insert(id) {
                        return Err(duplicate_compartment());
                    }
                    let effective = overrides
                        .and_then(|map| map.get(&id).cloned())
                        .unwrap_or_else(|| (*inner).clone());
                    compartments.insert(id, effective.clone());
                    walk(effective, prio, buckets, seen_p, seen_f, seen_c, compartments, overrides)?;
                }
            }
            Ok(())
        }
    
        walk(
            root,
            Priority::Default,
            &mut buckets,
            &mut seen_provider_ids,
            &mut seen_field_types,
            &mut seen_compartments,
            &mut compartments,
            compartment_overrides,
        )?;
    
        let mut providers = Vec::new();
        let mut fields = Vec::new();
        for item in buckets.into_iter().flatten() {
            match item {
                FlatItem::Provider(p) => providers.push(p),
                FlatItem::Field(f) => fields.push(f),
            }
        }
        Ok((providers, fields, compartments))
    }
}
