use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use mynox::Report;

use crate::extension::IntoExtension;
use crate::extension::utils::dep;
use crate::extension::Compartment;
use crate::extension::StateField;
use crate::extension::FacetDef;
use crate::extension::Single;
use crate::extension::Multi;
use crate::extension::Facet;
use crate::StateEffect;
use crate::Composable;
use crate::Selection;
use crate::Transaction;
use crate::Extension;
use crate::Priority;
use crate::State;
use crate::Text;
use crate::tr;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MultiNumeric(pub usize);
impl Facet for MultiNumeric {
    type Mode = Multi;
    fn compare_input(a: &Self, b: &Self) -> bool {
        a == b
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MultiStr(pub &'static str);
impl Facet for MultiStr {
    type Mode = Multi;
    fn compare_input(a: &Self, b: &Self) -> bool {
        a == b
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MultiString(pub String);
impl Facet for MultiString {
    type Mode = Multi;
    fn compare_input(a: &Self, b: &Self) -> bool {
        a == b
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct MultiBoolean(pub bool);
impl Facet for MultiBoolean {
    type Mode = Single;
    fn compare_input(a: &Self, b: &Self) -> bool {
        a == b
    }
}

fn mk<const N: usize>(extensions: [Extension; N]) -> State {
    State::from(Extension::from(extensions))
}

#[test]
fn allows_querying_of_facets() -> Result<(), Report> {
    let state = mk([
        Extension::from(MultiNumeric(10)),
        Extension::from(MultiNumeric(20)),
        Extension::from(MultiStr("x")),
        Extension::from(MultiStr("y")),
    ]);
    
    let facets: Vec<_> = state.facet::<MultiNumeric>().iter().map(|s| s.0).collect();
    assert_eq!(facets, vec![10, 20]);
    let facets: Vec<_> = state.facet::<MultiStr>().iter().map(|s| s.0).collect();
    assert_eq!(facets, vec![10, 20].iter().map(|_| "").zip(["x", "y"]).map(|(_, b)| b).collect::<Vec<_>>());
    Ok(())
}

#[test]
fn includes_sub_extenders() -> Result<(), Report> {
    fn e(s: &str) -> Extension {
        Extension::Group(vec![
            Extension::from(MultiNumeric(s.len())),
            Extension::from(MultiNumeric(s.parse().unwrap())),
        ])
    }
    
    let state = mk([
        Extension::from(MultiNumeric(5)), e("20"),
        Extension::from(MultiNumeric(40)), e("100"),
    ]);
    
    let facets: Vec<_> = state.facet::<MultiNumeric>().iter().map(|s| s.0).collect();
    assert_eq!(facets, vec![5, 2, 20, 40, 3, 100]);
    Ok(())
}

#[test]
fn only_includes_duplicated_extensions_once() -> Result<(), Report> {
    let e = Extension::from(MultiNumeric(50));
    let state = mk([
        Extension::from(MultiNumeric(1)), e.clone(),
        Extension::from(MultiNumeric(4)), e,
    ]);
    let facets: Vec<_> = state.facet::<MultiNumeric>().iter().map(|s| s.0).collect();
    assert_eq!(facets, vec![1, 50, 4]);
    Ok(())
}

#[test]
fn supports_compartments() -> Result<(), Report> {
    let comp = Compartment::new();
    let state = mk([
        MultiNumeric(1).into_ext(),
        comp.of(MultiNumeric(2)),
        MultiNumeric(3).into_ext(),
    ]);
    let facets: Vec<_> = state.facet::<MultiNumeric>().iter().map(|s| s.0).collect();
    assert_eq!(facets, vec![1, 2, 3]);

    let state2 = state
        .update(tr!({ effects: [comp.reconfigure(MultiNumeric(4))] }))?
        .state();
    let facets2: Vec<_> = state2.facet::<MultiNumeric>().iter().map(|s| s.0).collect();
    assert_eq!(facets2, vec![1, 4, 3]);
    Ok(())
}

#[test]
fn sorts_extensions_by_priority() -> Result<(), Report> {
    let state = mk([
        MultiStr("a").into_ext(),
        MultiStr("b").into_ext(),
        MultiStr("c").into_ext().with_priority(Priority::High),
        MultiStr("d").into_ext().with_priority(Priority::Highest),
        MultiStr("e").into_ext().with_priority(Priority::Low),
        MultiStr("f").into_ext().with_priority(Priority::High),
        MultiStr("g").into_ext(),
    ]);
    let facets: Vec<_> = state.facet::<MultiStr>().iter().map(|s| s.0).collect();
    assert_eq!(facets, vec!["d", "c", "f", "a", "b", "g", "e"]);
    Ok(())
}

#[test]
fn lets_sub_extensions_inherit_their_parents_priority() -> Result<(), Report> {
    let state = mk([
        MultiNumeric(1).into_ext(),
        Extension::from([
            MultiNumeric(2).into_ext(),
        ]).with_priority(Priority::Highest),
        MultiNumeric(4).into_ext(),
    ]);
    let facets: Vec<_> = state.facet::<MultiNumeric>().iter().map(|s| s.0).collect();
    assert_eq!(facets, vec![2, 1, 4]);
    Ok(())
}

#[test]
fn supports_dynamic_facet() -> Result<(), Report> {
    let state = mk([
        MultiNumeric(1).into_ext(),
        MultiNumeric::compute([], |_| { MultiNumeric(88) })
    ]);
    let facets: Vec<_> = state.facet::<MultiNumeric>().iter().map(|s| s.0).collect();
    assert_eq!(facets, vec![1, 88]);
    Ok(())
}

#[test]
fn only_recomputes_a_facet_value_when_necessary() -> Result<(), Report> {
    static COMPUTE_CALLS: AtomicUsize = AtomicUsize::new(0);
    let state = mk([
        MultiNumeric(1).into_ext(),
        MultiNumeric::compute([dep::<MultiStr>()], |s| {
            COMPUTE_CALLS.fetch_add(1, Ordering::SeqCst);
            let len = s.facet::<MultiStr>().iter().map(|x| x.0.len()).sum();
            MultiNumeric(len)
        }),
        MultiStr("hello").into_ext(),
    ]);
    
    let facets: Vec<_> = state.facet::<MultiNumeric>().iter().map(|s| s.0).collect();
    assert_eq!(facets, vec![1, 5]);
    assert_eq!(COMPUTE_CALLS.load(Ordering::SeqCst), 1);
    
    let tr = Transaction::default();
    let state = state.apply(&tr);
    let facets_second: Vec<_> = state.facet::<MultiNumeric>().iter().map(|s| s.0).collect();
    
    assert!(facets == facets_second);
    assert_eq!(COMPUTE_CALLS.load(Ordering::SeqCst), 1);
    Ok(())
}

#[test]
fn can_handle_dependencies_on_facets_that_arent_present_in_the_state() -> Result<(), Report> {
    let state = mk([
        MultiNumeric::compute([dep::<MultiStr>()], |s| {
            let len = s.facet::<MultiStr>().iter().map(|x| x.0.len()).sum();
            MultiNumeric(len)
        }),
        MultiString::compute([dep::<MultiBoolean>()], |s| {
            let string = s.facet::<MultiBoolean>().0.to_string();
            MultiString(string)
        })
    ]);
    let tr = Transaction::default();
    let state = state.apply(&tr);
    let facets: Vec<_> = state.facet::<MultiNumeric>().iter().map(|s| s.0).collect();
    assert_eq!(facets, vec![0]);
    Ok(())
}

#[test]
fn can_specify_a_dependency_on_the_document() -> Result<(), Report> {
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let state = State::default().with_extensions(MultiNumeric::compute([dep::<Text>()], |s| {
        COUNT.fetch_add(1, Ordering::SeqCst);
        MultiNumeric(s.doc().len_chars())
    }));

    assert_eq!(state.facet::<MultiNumeric>(), vec![MultiNumeric(0)]);
    assert_eq!(COUNT.load(Ordering::SeqCst), 1);

    let state = state
        .update(tr!({ changes: { insert: "hello", from: 0 } }))?
        .state();
    assert_eq!(state.facet::<MultiNumeric>(), vec![MultiNumeric(5)]);
    assert_eq!(COUNT.load(Ordering::SeqCst), 2);

    let state = state.update(tr!({ selection: Selection::cursor(2) }))?.state();
    assert_eq!(state.facet::<MultiNumeric>(), vec![MultiNumeric(5)]);
    assert_eq!(COUNT.load(Ordering::SeqCst), 2);
    Ok(())
}

#[test]
fn can_specify_a_dependency_on_the_selection() -> Result<(), Report> {
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let state = State::default().with_extensions(MultiNumeric::compute([dep::<Selection>()], |s| {
        COUNT.fetch_add(1, Ordering::SeqCst);
        MultiNumeric(s.selection().main().from())
    }));

    assert_eq!(state.facet::<MultiNumeric>(), vec![MultiNumeric(0)]);
    assert_eq!(COUNT.load(Ordering::SeqCst), 1);

    let state = state
        .update(tr!({ changes: { insert: "hello", from: 0 } }))?
        .state();
    assert_eq!(state.facet::<MultiNumeric>(), vec![MultiNumeric(5)]);
    assert_eq!(COUNT.load(Ordering::SeqCst), 2);

    let state = state.update(tr!({ selection: Selection::cursor(2) }))?.state();
    assert_eq!(state.facet::<MultiNumeric>(), vec![MultiNumeric(2)]);
    assert_eq!(COUNT.load(Ordering::SeqCst), 3);

    let state = state.update(tr!())?.state();
    assert_eq!(state.facet::<MultiNumeric>(), vec![MultiNumeric(2)]);
    assert_eq!(COUNT.load(Ordering::SeqCst), 3);
    Ok(())
}

#[test]
fn can_provide_multiple_values_at_once() -> Result<(), Report> {
    let state = State::from("hello").with_extensions(Extension::from([
        MultiNumeric::compute_multi([dep::<Text>()], |s| {
            if s.doc().len_chars() % 2 == 1 {
                vec![MultiNumeric(100), MultiNumeric(10)]
            } else {
                vec![]
            }
        }),
        MultiNumeric(1).into_ext(),
    ]));

    let facets: Vec<_> = state.facet::<MultiNumeric>().iter().map(|s| s.0).collect();
    assert_eq!(facets, vec![100, 10, 1]);

    let state = state
        .update(tr!({ changes: { insert: "!", from: 0 } }))?
        .state();
    let facets2: Vec<_> = state.facet::<MultiNumeric>().iter().map(|s| s.0).collect();
    assert_eq!(facets2, vec![1]);
    Ok(())
}

#[test]
fn works_with_a_static_combiner() -> Result<(), Report> {
    struct SumFacet;
    impl FacetDef for SumFacet {
        type Input = usize;
        type Output = usize;
        fn combine(inputs: &[usize]) -> usize {
            inputs.iter().sum()
        }
        fn compare_input(a: &usize, b: &usize) -> bool {
            a == b
        }
        fn compare(a: &usize, b: &usize) -> bool {
            a == b
        }
    }

    let state = State::from("hello").with_extensions(Extension::from([
        SumFacet::compute([dep::<Text>()], |s| s.doc().len_chars()),
        Extension::facet::<SumFacet>(2),
        Extension::facet::<SumFacet>(3),
    ]));

    assert_eq!(state.facet::<SumFacet>(), 10);
    Ok(())
}

#[test]
fn facet_from_field() -> Result<(), Report> {
    struct NumField;
    impl StateField for NumField {
        type Value = usize;
        fn create(_: &State) -> usize {
            2
        }
        fn update(val: &usize, tr: &Transaction) -> usize {
            if tr.doc_changed() { val + 1 } else { *val }
        }
        fn compare(a: &usize, b: &usize) -> bool {
            a == b
        }
    }

    let state = State::default().with_extensions(Extension::from([
        NumField::init(),
        MultiNumeric::from_field_with::<NumField>(|v| MultiNumeric(*v)),
        MultiNumeric::from_field_multi::<NumField>(|v| {
            vec![MultiNumeric(*v + 1), MultiNumeric(*v + 2)]
        }),
    ]));

    let facets: Vec<_> = state.facet::<MultiNumeric>().iter().map(|s| s.0).collect();
    assert_eq!(facets, vec![2, 3, 4]);

    let state = state
        .update(tr!({ changes: { insert: "!", from: 0 } }))?
        .state();
    let facets2: Vec<_> = state.facet::<MultiNumeric>().iter().map(|s| s.0).collect();
    assert_eq!(facets2, vec![3, 4, 5]);
    Ok(())
}

#[test]
fn state_field_can_provide_facets_directly() -> Result<(), Report> {
    struct SelfProvidingField;
    impl StateField for SelfProvidingField {
        type Value = MultiNumeric;
        fn create(_: &State) -> MultiNumeric {
            MultiNumeric(42)
        }
        fn update(val: &MultiNumeric, tr: &Transaction) -> MultiNumeric {
            if tr.doc_changed() {
                MultiNumeric(val.0 + 1)
            } else {
                *val
            }
        }
        fn compare(a: &MultiNumeric, b: &MultiNumeric) -> bool {
            a == b
        }
        fn provide() -> Option<Extension> {
            Some(MultiNumeric::from_field::<SelfProvidingField>())
        }
    }

    // Even if SelfProvidingField::init() is included twice, both field and provided facet are deduplicated
    let state = mk([SelfProvidingField::init(), SelfProvidingField::init()]);
    assert_eq!(state.facet::<MultiNumeric>(), vec![MultiNumeric(42)]);

    let state = state
        .update(tr!({ changes: { from: 0, insert: "x" } }))?
        .state();
    assert_eq!(state.facet::<MultiNumeric>(), vec![MultiNumeric(43)]);
    Ok(())
}

#[test]
fn survives_reconfiguration() -> Result<(), Report> {
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let comp = Compartment::new();
    let state = mk([
        MultiNumeric::compute([dep::<Text>(), dep::<MultiStr>()], |s| {
            COUNT.fetch_add(1, Ordering::SeqCst);
            let str_len: usize = s.facet::<MultiStr>().iter().map(|x| x.0.len()).sum();
            MultiNumeric(s.doc().len_chars() + str_len)
        }),
        MultiNumeric(2).into_ext(),
        comp.of(MultiStr("a")),
    ]);

    let facets: Vec<_> = state.facet::<MultiNumeric>().iter().map(|s| s.0).collect();
    assert_eq!(facets, vec![1, 2]);
    assert_eq!(COUNT.load(Ordering::SeqCst), 1);

    let state = state
        .update(tr!({
            effects: [StateEffect::append_config(MultiNumeric(3))]
        }))?
        .state();
    let facets2: Vec<_> = state.facet::<MultiNumeric>().iter().map(|s| s.0).collect();
    assert_eq!(facets2, vec![1, 2, 3]);
    assert_eq!(COUNT.load(Ordering::SeqCst), 1);

    let state = state
        .update(tr!({
            effects: [comp.reconfigure(MultiStr("bc"))]
        }))?
        .state();
    let facets3: Vec<_> = state.facet::<MultiNumeric>().iter().map(|s| s.0).collect();
    assert_eq!(facets3, vec![2, 2, 3]);
    assert_eq!(COUNT.load(Ordering::SeqCst), 2);
    Ok(())
}

#[test]
fn tracks_duplicated_fields() -> Result<(), Report> {
    static CREATED: AtomicUsize = AtomicUsize::new(0);
    struct DupField;
    impl StateField for DupField {
        type Value = usize;
        fn create(_: &State) -> usize {
            CREATED.fetch_add(1, Ordering::SeqCst) + 1
        }
        fn update(val: &usize, _: &Transaction) -> usize {
            *val
        }
    }

    let state = mk([DupField::init(), DupField::init()]);
    assert_eq!(state.field::<DupField>(), Some(&1));
    assert_eq!(CREATED.load(Ordering::SeqCst), 1);
    Ok(())
}

#[test]
#[should_panic(expected = "Cyclic dependency detected in dynamic facets")]
fn throws_on_cyclic_dependencies() {
    let _state = mk([
        MultiNumeric::compute([dep::<MultiString>()], |s| {
            MultiNumeric(s.facet::<MultiString>().len())
        }),
        MultiString::compute([dep::<MultiNumeric>()], |s| {
            MultiString(s.facet::<MultiNumeric>().len().to_string())
        }),
    ]);
}

#[test]
fn dynamic_facet_depends_on_doc_and_selection() -> Result<(), Report> {
    static DOC_CALLS: AtomicUsize = AtomicUsize::new(0);
    static SEL_CALLS: AtomicUsize = AtomicUsize::new(0);

    let state = State::from("hello").with_extensions(Extension::from([
        MultiNumeric::compute([dep::<Text>()], |s| {
            DOC_CALLS.fetch_add(1, Ordering::SeqCst);
            MultiNumeric(s.doc().len_chars())
        }),
        MultiString::compute([dep::<Selection>()], |s| {
            SEL_CALLS.fetch_add(1, Ordering::SeqCst);
            MultiString(format!("pos:{}", s.selection().main().head()))
        }),
    ]));

    assert_eq!(state.facet::<MultiNumeric>(), vec![MultiNumeric(5)]);
    assert_eq!(state.facet::<MultiString>(), vec![MultiString("pos:0".to_string())]);
    assert_eq!(DOC_CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(SEL_CALLS.load(Ordering::SeqCst), 1);

    // 1. Selección cambia pero doc NO cambia -> solo se recalcula la faceta dependiente de Selection
    let state = state.update(tr!({ selection: Selection::cursor(3) }))?.state();
    assert_eq!(state.facet::<MultiNumeric>(), vec![MultiNumeric(5)]);
    assert_eq!(state.facet::<MultiString>(), vec![MultiString("pos:3".to_string())]);
    assert_eq!(DOC_CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(SEL_CALLS.load(Ordering::SeqCst), 2);

    // 2. Doc cambia -> se recalculan ambas (doc y selection)
    let state = state.update(tr!({ changes: { from: 3, to: 3, insert: "!" } }))?.state();
    assert_eq!(state.facet::<MultiNumeric>(), vec![MultiNumeric(6)]);
    assert_eq!(state.facet::<MultiString>(), vec![MultiString("pos:4".to_string())]);
    assert_eq!(DOC_CALLS.load(Ordering::SeqCst), 2);
    assert_eq!(SEL_CALLS.load(Ordering::SeqCst), 3);

    Ok(())
}

#[test]
fn compare_prevents_downstream_facet_recomputation_and_topological_order_works() -> Result<(), Report> {
    static DOWNSTREAM_CALLS: AtomicUsize = AtomicUsize::new(0);

    struct DocParityField;
    impl StateField for DocParityField {
        type Value = bool;
        fn create(state: &State) -> Self::Value {
            state.doc().len_chars() % 2 == 0
        }
        fn update(_value: &Self::Value, tx: &Transaction) -> Self::Value {
            if let Some(ch) = &tx.changes {
                ch.len_new() % 2 == 0
            } else {
                *_value
            }
        }
        fn compare(a: &Self::Value, b: &Self::Value) -> bool {
            a == b
        }
    }

    // Declaramos MultiString (downstream) ANTES que MultiBoolean (upstream) para verificar orden topológico
    let state = State::from("abcd").with_extensions(Extension::from([
        DocParityField::init(),
        MultiString::compute([dep::<MultiBoolean>()], |s| {
            DOWNSTREAM_CALLS.fetch_add(1, Ordering::SeqCst);
            MultiString(format!("even:{}", s.facet::<MultiBoolean>().0))
        }),
        MultiBoolean::compute([dep::<DocParityField>()], |s| {
            MultiBoolean(*s.field::<DocParityField>().unwrap())
        }),
    ]));

    assert_eq!(state.field::<DocParityField>(), Some(&true));
    assert_eq!(state.facet::<MultiBoolean>(), MultiBoolean(true));
    assert_eq!(state.facet::<MultiString>(), vec![MultiString("even:true".to_string())]);
    assert_eq!(DOWNSTREAM_CALLS.load(Ordering::SeqCst), 1);

    // Insertamos 2 caracteres ("ef"): la longitud pasa de 4 a 6 (sigue siendo par -> DocParityField no cambia gracias a compare)
    let state = state.update(tr!({ changes: { from: 4, to: 4, insert: "ef" } }))?.state();
    assert_eq!(state.facet::<MultiString>(), vec![MultiString("even:true".to_string())]);
    assert_eq!(DOWNSTREAM_CALLS.load(Ordering::SeqCst), 1);

    // Insertamos 1 carácter ("!"): la longitud pasa de 6 a 7 (impar -> cambia y propaga a MultiBoolean y MultiString)
    let state = state.update(tr!({ changes: { from: 6, to: 6, insert: "!" } }))?.state();
    assert_eq!(state.facet::<MultiString>(), vec![MultiString("even:false".to_string())]);
    assert_eq!(DOWNSTREAM_CALLS.load(Ordering::SeqCst), 2);

    Ok(())
}
