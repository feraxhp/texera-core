use mynox::Report;

use crate::extension::IntoExtension;
use crate::extension::StateField;
use crate::extension::Compartment;
use crate::extension::FacetDef;
use crate::extension::Multi;
use crate::extension::Facet;
use crate::extension::dep;
use crate::AllowMultipleSelections;
use crate::TransactionExtender;
use crate::TransactionFilter;
use crate::AnnotationValue;
use crate::LineSeparator;
use crate::CharCategory;
use crate::ChangeFilter;
use crate::StateEffect;
use crate::Transaction;
use crate::Annotation;
use crate::ChangeDesc;
use crate::ChangeSpec;
use crate::Composable;
use crate::Extension;
use crate::Selection;
use crate::WordChars;
use crate::MapMode;
use crate::ReadOnly;
use crate::TabSize;
use crate::Effect;
use crate::Assoc;
use crate::State;
use crate::tr;


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SomeAnnotation(usize);
impl AnnotationValue for SomeAnnotation {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NumFacet(usize);
impl Facet for NumFacet {
    type Mode = Multi;
    fn compare_input(a: &Self, b: &Self) -> bool {
        a == b
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NumEffect(usize);
impl Effect for NumEffect {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PosEffect(usize);
impl Effect for PosEffect {
    fn map(&self, changes: &ChangeDesc) -> Option<Self> {
        changes
            .map_pos(self.0, Assoc::After, MapMode::TrackDel)
            .ok()
            .flatten()
            .map(PosEffect)
    }
}


#[test]
fn holds_doc_and_selection_properties() {
    let state = State::from("hello");
    
    assert_eq!(&state.doc().to_string(), "hello");
    assert_eq!(state.selection().main().from(), 0 as usize);
}

#[test]
fn can_apply_changes() -> Result<(), Report> {
    let state = State::from("hello");

    let tr = tr!({
        changes: [
            {from: 2, to: 4, insert: "w"}, 
            {from: 5, to: 5, insert: "!"},
        ]
    });
    
    let transaction = state.update(tr)?;
    assert_eq!(transaction.state().doc().to_string(), "hewo!");
    assert_eq!(transaction.start_state().doc().to_string(), "hello");
    Ok(())
}

#[test]
fn apply_changes() -> Result<(), Report> {
    let state = State::from("hello");
    
    let transaction = state.update(tr!( { changes: { from: 2, to: 2, insert: "a" } }));
    let transaction = transaction?;
    assert_eq!(transaction.state().doc().to_string(), "heallo");
    assert_eq!(transaction.start_state().doc().to_string(), "hello");
    Ok(())
}

#[test]
fn apply_changes_start_and_end() -> Result<(), Report> {
    let state = State::from("world");

    let tr = tr!({
        changes: [
            {from: 0, to: 0, insert: "hello "},
            {from: 5, to: 5, insert: "!"},
        ]
    });

    let transaction = state.update(tr)?;
    
    assert_eq!(transaction.state().doc().to_string(), "hello world!");
    assert_eq!(transaction.start_state().doc().to_string(), "world");
    Ok(())
}

#[test]
fn apply_delete_all() -> Result<(), Report> {
    let state = State::from("goodbye");

    let tr = tr!({
        changes: { from: 0, to: 7, }
    });

    let transaction = state.update(tr)?;
    
    assert_eq!(transaction.state().doc().to_string(), "");
    assert_eq!(transaction.start_state().doc().to_string(), "goodbye");
    Ok(())
}

#[test]
fn maps_selection_through_changes() -> Result<(), Report> {
    let state = State::with_selection(
        "abcdefgh",
        Selection::from([0, 4, 8].map(|n| Selection::cursor(n))),
    )
    .with_extensions(AllowMultipleSelections(true));

    let tr = state.replace_selection("Q");
    let transaction = state.update(tr)?;
    let new_state = transaction.state();
    
    assert_eq!(new_state.doc().to_string(), "QabcdQefghQ");
    assert_eq!(transaction.start_state().doc().to_string(), "abcdefgh");

    let sel: String = new_state
        .iter_selections()
        .map(|(sel, _)| format!("{}/", sel.from()))
        .collect();
    
    assert_eq!("1/6/11/", sel);
    Ok(())
}

#[test]
fn can_store_annotations_on_transactions() -> Result<(), Report> {
    let tr = State::from("foo").update(tr!({
        annotations: [Annotation::new(SomeAnnotation(55))]
    }))?;
    
    assert_eq!(tr.annotation::<SomeAnnotation>(), Some(&SomeAnnotation(55)));
    Ok(())
}

#[test]
fn throws_when_a_changes_bounds_are_invalid() {
    let state = State::from("1234");
    
    assert!(ChangeSpec::new(2, Some(1), None).is_err());
    
    let out_of_bounds = ChangeSpec::new(2, Some(10), Some("x".into())).unwrap();
    let tr = crate::TransactionSpec::default().with_change(out_of_bounds);
    assert!(state.update(tr).is_err());
}

#[test]
fn stores_and_updates_tab_size() -> Result<(), Report> {
    let deflt = State::default();
    let two = State::from(Extension::from(TabSize(2)));
    
    assert_eq!(deflt.tab_size(), 4);
    assert_eq!(two.tab_size(), 2);
    
    let updated = deflt
        .update(tr!({
            effects: [StateEffect::reconfigure(TabSize(8))]
        }))?
        .state();
    assert_eq!(updated.tab_size(), 8);
    Ok(())
}

#[test]
fn stores_and_updates_the_line_separator() -> Result<(), Report> {
    let deflt = State::default();
    let crlf = State::from(Extension::from(LineSeparator("\r\n")));
    
    assert_eq!(deflt.facet::<LineSeparator>(), None);
    assert_eq!(deflt.line_break(), "\n");
    assert_eq!(deflt.to_text("a\nb").len_lines(), 2);
    
    assert_eq!(crlf.facet::<LineSeparator>(), Some("\r\n"));
    assert_eq!(crlf.line_break(), "\r\n");
    assert_eq!(crlf.to_text("a\nb").len_lines(), 1);
    
    let updated = crlf
        .update(tr!({
            effects: [StateEffect::reconfigure(LineSeparator("\n"))]
        }))?
        .state();
    assert_eq!(updated.facet::<LineSeparator>(), Some("\n"));
    Ok(())
}

#[test]
fn stores_and_updates_fields() -> Result<(), Report> {
    struct Field1;
    impl StateField for Field1 {
        type Value = usize;
        fn create(_: &State) -> usize { 0 }
        fn update(val: &usize, _: &Transaction) -> usize { val + 1 }
    }

    struct Field2;
    impl StateField for Field2 {
        type Value = usize;
        fn create(state: &State) -> usize { state.field::<Field1>().unwrap() + 10 }
        fn update(val: &usize, _: &Transaction) -> usize { *val }
    }

    let state = State::from(Extension::from([Field1::init(), Field2::init()]));
    assert_eq!(state.field::<Field1>(), Some(&0));
    assert_eq!(state.field::<Field2>(), Some(&10));

    let new_state = state.update(tr!())?.state();
    assert_eq!(new_state.field::<Field1>(), Some(&1));
    assert_eq!(new_state.field::<Field2>(), Some(&10));
    Ok(())
}

#[test]
fn allows_fields_to_have_an_initializer() -> Result<(), Report> {
    struct Field;
    impl StateField for Field {
        type Value = usize;
        fn create(_: &State) -> usize { 0 }
        fn update(val: &usize, _: &Transaction) -> usize { val + 1 }
    }

    let state = State::from(Field::init_with(|_| 10));
    assert_eq!(state.field::<Field>(), Some(&10));
    assert_eq!(state.update(tr!())?.state().field::<Field>(), Some(&11));
    Ok(())
}

#[test]
fn can_preserve_fields_across_reconfiguration() -> Result<(), Report> {
    struct Field;
    impl StateField for Field {
        type Value = usize;
        fn create(_: &State) -> usize { 0 }
        fn update(val: &usize, _: &Transaction) -> usize { val + 1 }
    }

    let start = State::from(Field::init()).update(tr!())?.state();
    assert_eq!(start.field::<Field>(), Some(&1));

    let reconf = start
        .clone()
        .update(tr!({ effects: [StateEffect::reconfigure(Field::init())] }))?
        .state();
    assert_eq!(reconf.field::<Field>(), Some(&2));

    let dropped = start
        .update(tr!({ effects: [StateEffect::reconfigure(Extension::default())] }))?
        .state();
    assert_eq!(dropped.field::<Field>(), None);
    Ok(())
}

#[test]
fn can_replace_extension_groups() -> Result<(), Report> {
    let comp = Compartment::new();
    let content = Extension::from(NumFacet(10));
    let state = State::from(Extension::from([
        comp.of(content.clone()),
        NumFacet(20).into_ext(),
    ]));

    assert_eq!(comp.get(&state), Some(&content));
    let nums: Vec<_> = state.facet::<NumFacet>().iter().map(|n| n.0).collect();
    assert_eq!(nums, vec![10, 20]);

    let content2 = Extension::from([NumFacet(1).into_ext(), NumFacet(2).into_ext()]);
    let state2 = state
        .update(tr!({ effects: [comp.reconfigure(content2.clone())] }))?
        .state();
    assert_eq!(comp.get(&state2), Some(&content2));
    let nums2: Vec<_> = state2.facet::<NumFacet>().iter().map(|n| n.0).collect();
    assert_eq!(nums2, vec![1, 2, 20]);

    let state3 = state2
        .update(tr!({ effects: [comp.reconfigure(NumFacet(3))] }))?
        .state();
    let nums3: Vec<_> = state3.facet::<NumFacet>().iter().map(|n| n.0).collect();
    assert_eq!(nums3, vec![3, 20]);
    Ok(())
}

#[test]
fn raises_an_error_on_duplicate_extension_groups() {
    let comp = Compartment::new();

    let res1 = State::default().try_with_extensions(Extension::from([
        comp.of(NumFacet(1)),
        comp.of(NumFacet(2)),
    ]));
    assert!(res1.is_err());

    let res2 = State::default().try_with_extensions(comp.of(comp.of(NumFacet(1))));
    assert!(res2.is_err());
}

#[test]
fn preserves_compartments_on_reconfigure() -> Result<(), Report> {
    let comp = Compartment::new();
    let init = comp.of(NumFacet(10));
    let state = State::from(Extension::from([init.clone(), NumFacet(20).into_ext()]));

    let state = state
        .update(tr!({ effects: [comp.reconfigure(NumFacet(0))] }))?
        .state();
    let nums: Vec<_> = state.facet::<NumFacet>().iter().map(|n| n.0).collect();
    assert_eq!(nums, vec![0, 20]);

    let state = state
        .update(tr!({
            effects: [StateEffect::reconfigure(Extension::from([
                init,
                NumFacet(2).into_ext(),
            ]))]
        }))?
        .state();
    let nums: Vec<_> = state.facet::<NumFacet>().iter().map(|n| n.0).collect();
    assert_eq!(nums, vec![0, 2]);
    Ok(())
}

#[test]
fn forgets_dropped_compartments() -> Result<(), Report> {
    let comp = Compartment::new();
    let init = comp.of(NumFacet(10));
    let state = State::from(Extension::from([init.clone(), NumFacet(20).into_ext()]));

    let state = state
        .update(tr!({ effects: [comp.reconfigure(NumFacet(0))] }))?
        .state();
    let nums: Vec<_> = state.facet::<NumFacet>().iter().map(|n| n.0).collect();
    assert_eq!(nums, vec![0, 20]);

    let state = state
        .update(tr!({ effects: [StateEffect::reconfigure(NumFacet(2))] }))?
        .state();
    let nums: Vec<_> = state.facet::<NumFacet>().iter().map(|n| n.0).collect();
    assert_eq!(nums, vec![2]);
    assert_eq!(comp.get(&state), None);

    let state = state
        .update(tr!({
            effects: [StateEffect::reconfigure(Extension::from([
                init,
                NumFacet(2).into_ext(),
            ]))]
        }))?
        .state();
    let nums: Vec<_> = state.facet::<NumFacet>().iter().map(|n| n.0).collect();
    assert_eq!(nums, vec![10, 2]);
    Ok(())
}

#[test]
fn allows_facets_computed_from_fields() -> Result<(), Report> {
    struct Field;
    impl StateField for Field {
        type Value = Vec<usize>;
        fn create(_: &State) -> Vec<usize> {
            vec![0]
        }
        fn update(v: &Vec<usize>, tr: &Transaction) -> Vec<usize> {
            if tr.doc_changed() {
                vec![tr.doc().len_chars()]
            } else {
                v.clone()
            }
        }
        fn compare(a: &Vec<usize>, b: &Vec<usize>) -> bool {
            a == b
        }
    }

    let state = State::from(Extension::from([
        Field::init(),
        NumFacet::compute([dep::<Field>()], |state| {
            NumFacet(state.field::<Field>().unwrap()[0])
        }),
        NumFacet(1).into_ext(),
    ]));

    let nums: Vec<_> = state.facet::<NumFacet>().iter().map(|n| n.0).collect();
    assert_eq!(nums, vec![0, 1]);

    let state2 = state.clone().update(tr!())?.state();
    assert_eq!(state2.facet::<NumFacet>(), state.facet::<NumFacet>());

    let state3 = state
        .update(tr!({ changes: { insert: "hi", from: 0 } }))?
        .state();
    let nums3: Vec<_> = state3.facet::<NumFacet>().iter().map(|n| n.0).collect();
    assert_eq!(nums3, vec![2, 1]);
    Ok(())
}

#[test]
fn blocks_multiple_selections_when_not_allowed() -> Result<(), Report> {
    let cursors = Selection::from([Selection::cursor(0), Selection::cursor(1)]);
    let state = State::with_selection("123", cursors.clone());

    assert_eq!(state.selection().ranges_ref().len(), 1);
    assert_eq!(
        state
            .update(tr!({ selection: cursors }))?
            .state()
            .selection()
            .ranges_ref()
            .len(),
        1
    );
    Ok(())
}

#[test]
fn can_make_simple_changes() -> Result<(), Report> {
    let state = State::from("hi");
    
    let tr = state.change_by_range(|(r, _)| {
        Ok(tr!({
            changes: { from: r.from(), to: r.from() + 1, insert: "q" },
            selection: Selection::cursor(r.from() + 1)
        }))
    })?;

    let state = state.update(tr)?.state();
    
    assert_eq!(state.doc().to_string(), "qi");
    assert_eq!(state.selection().main().from(), 1);
    Ok(())
}

#[test]
fn does_the_right_thing_when_there_are_multiple_selections() -> Result<(), Report> {
    let state = State::with_selection(
        "1 2 3 4",
        Selection::from([
            Selection::range(0, 1),
            Selection::range(2, 3),
            Selection::range(4, 5),
            Selection::range(6, 7),
        ]),
    )
    .with_extensions(AllowMultipleSelections(true));
    
    let tr = state.change_by_range(|(r, _)| {
        Ok(tr!({
            changes: { from: r.from(), to: r.to(), insert: &"-".repeat((r.from() >> 1) + 1) },
            selection: Selection::range(r.from(), r.from() + 1 + (r.from() >> 1))
        }))
    })?;
    
    let transaction = state.update(tr)?;
    let new_state = transaction.state();
    
    assert_eq!(&new_state.doc().to_string(), "- -- --- ----");
    let expected_ranges = "0-1 2-4 5-8 9-13 ";
    let ranges: String = new_state
        .iter_selections()
        .map(|(sel, _)| format!("{}-{} ", sel.from(), sel.to()))
        .collect();
    
    assert_eq!(&ranges, expected_ranges);
    Ok(())
}

#[test]
fn can_cancel_changes() -> Result<(), Report> {
    let state = State::from("one two").with_extensions(ChangeFilter::new(|tr| {
        tr.changes().len_new() <= tr.changes().len_old()
    }));

    let tr1 = state.clone().update(tr!({
        changes: { from: 3, insert: " three" },
        selection: Selection::cursor(13),
    }))?;
    assert_eq!(tr1.state().doc().to_string(), "one two");
    assert_eq!(tr1.state().selection().main().head(), 7);

    let tr2 = state.update(tr!({
        changes: { from: 4, to: 7, insert: "2" }
    }))?;
    assert_eq!(tr2.state().doc().to_string(), "one 2");
    Ok(())
}

#[test]
fn can_split_changes() -> Result<(), Report> {
    let state = State::from("onetwo").with_extensions(ChangeFilter::new(|tr| {
        let len = tr.start_state().doc().len_chars();
        [len / 3, (2 * len) / 3]
    }));

    let res = state.update(tr!({ changes: { from: 0, to: 6 } }))?;
    assert_eq!(res.state().doc().to_string(), "et");
    Ok(())
}

#[test]
fn combines_filter_masks() -> Result<(), Report> {
    let state = State::from("onetwo").with_extensions(Extension::from([
        ChangeFilter::new(|_| [0, 2]),
        ChangeFilter::new(|_| [4, 6]),
    ]));

    let res = state.update(tr!({ changes: { from: 0, to: 6 } }))?;
    assert_eq!(res.state().doc().to_string(), "onwo");
    Ok(())
}

#[test]
fn can_be_turned_off() -> Result<(), Report> {
    let state = State::from(ChangeFilter::new(|_| false));

    assert_eq!(
        state
            .clone()
            .update(tr!({ changes: { from: 0, insert: "hi" } }))?
            .state()
            .doc()
            .len_chars(),
        0
    );
    assert_eq!(
        state
            .update(tr!({ changes: { from: 0, insert: "hi" }, filter: false }))?
            .state()
            .doc()
            .len_chars(),
        2
    );
    Ok(())
}

#[test]
fn can_constrain_the_selection() -> Result<(), Report> {
    let state = State::from("one two").with_extensions(TransactionFilter::new(|tr| {
        if let Some(sel) = tr.selection()
            && sel.main().to() > 4
        {
            vec![tr.spec().clone(), tr!({ selection: Selection::cursor(4) })]
        } else {
            vec![tr.spec().clone()]
        }
    }));

    assert_eq!(
        state
            .clone()
            .update(tr!({ selection: Selection::cursor(3) }))?
            .state()
            .selection()
            .main()
            .to(),
        3
    );
    assert_eq!(
        state
            .update(tr!({ selection: Selection::cursor(7) }))?
            .state()
            .selection()
            .main()
            .to(),
        4
    );
    Ok(())
}

#[test]
fn can_append_sequential_changes() -> Result<(), Report> {
    let state = State::from("one two").with_extensions(TransactionFilter::new(|tr| {
        let new_len = tr.changes().len_new();
        Ok(vec![
            tr.spec().clone(),
            tr!({
                changes: { from: new_len, insert: "!" },
                sequential: true,
            }),
        ])
    }));

    let res = state.update(tr!({ changes: { from: 3, insert: "," } }))?;
    assert_eq!(res.state().doc().to_string(), "one, two!");
    Ok(())
}

#[test]
fn can_add_annotations() -> Result<(), Report> {
    let state = State::from(TransactionExtender::new(|_| {
        tr!({ annotations: [Annotation::new(SomeAnnotation(100))] })
    }));

    let tr = state.clone().update(tr!({ changes: { from: 0, insert: "!" } }))?;
    assert_eq!(tr.annotation::<SomeAnnotation>(), Some(&SomeAnnotation(100)));

    let tr_no_filter = state.update(tr!({
        changes: { from: 0, insert: "!" },
        filter: false,
    }))?;
    assert_eq!(
        tr_no_filter.annotation::<SomeAnnotation>(),
        Some(&SomeAnnotation(100))
    );
    Ok(())
}

#[test]
fn allows_multiple_extenders_to_take_effect() -> Result<(), Report> {
    let state = State::from(Extension::from([
        TransactionExtender::new(|_| tr!({ effects: [StateEffect::new(NumEffect(1))] })),
        TransactionExtender::new(|_| tr!({ effects: [StateEffect::new(NumEffect(2))] })),
    ]));

    let tr = state.update(tr!({ scroll_into_view: true }))?;
    let effs: Vec<usize> = tr
        .effects()
        .iter()
        .map(|e| e.get::<NumEffect>().map(|n| n.0).unwrap_or(0))
        .collect();
    assert_eq!(effs, vec![2, 1]);
    Ok(())
}

#[test]
fn finds_words_at_positions_and_uses_custom_word_chars() {
    let state = State::from("hello $foo-bar world");
    assert_eq!(state.word_at(2), Some(Selection::range(0, 5)));
    assert_eq!(state.word_at(5), Some(Selection::range(0, 5)));
    assert_eq!(state.word_at(6), None);
    assert_eq!(state.word_at(8), Some(Selection::range(7, 10)));

    let custom = state.with_extensions(WordChars::from("$-"));
    let cat = custom.char_categorizer(0);
    assert_eq!(cat.of('$'), CharCategory::Word);
    assert_eq!(cat.of('-'), CharCategory::Word);
    assert_eq!(custom.word_at(8), Some(Selection::range(6, 14)));
}

#[test]
fn accepts_multiple_transaction_specs_and_maps_effects() -> Result<(), Report> {
    let state = State::from("abcd");
    let res = state.update([
        tr!({
            changes: { from: 0, insert: "XX" },
            effects: [StateEffect::new(PosEffect(4))],
        }),
        tr!({
            changes: { from: 1, insert: "YY" },
            effects: [StateEffect::new(PosEffect(4))],
        }),
    ])?;

    assert_eq!(res.state_ref().doc().to_string(), "XXaYYbcd");
    // 1st spec's PosEffect(4) (in "XXabcd", i.e. 'c') is shifted by 2nd spec's "YY" at 1 -> 6
    let mapped_first = res.effects()[0].get::<PosEffect>().unwrap().0;
    assert_eq!(mapped_first, 6);
    // 2nd spec's PosEffect(4) (in "aYYbcd", i.e. 'c') is shifted by 1st spec's "XX" at 0 -> 6
    let mapped_second = res.effects()[1].get::<PosEffect>().unwrap().0;
    assert_eq!(mapped_second, 6);
    Ok(())
}

#[test]
fn exposes_annotations_user_events_and_inverted_changes_on_transaction() -> Result<(), Report> {
    struct EventTrackerField;
    impl StateField for EventTrackerField {
        type Value = (bool, Option<usize>);
        fn create(_: &State) -> Self::Value {
            (false, None)
        }
        fn update(_: &Self::Value, tx: &Transaction) -> Self::Value {
            let is_type = tx.is_user_event("input") && tx.is_user_event("input.type") && !tx.is_user_event("delete");
            let ann = tx.annotation::<SomeAnnotation>().map(|a| a.0);
            (is_type, ann)
        }
    }

    let state = State::from("hello world").with_extensions(EventTrackerField::init());
    let res = state.update(tr!({
        changes: { from: 5, to: 11, insert: " rust" },
        user_event: "input.type.compose",
        annotations: [Annotation::new(SomeAnnotation(77))],
    }))?;

    assert_eq!(res.state_ref().field::<EventTrackerField>(), Some(&(true, Some(77))));
    assert!(res.is_user_event("input"));
    assert!(res.is_user_event("input.type"));
    assert!(res.is_user_event("input.type.compose"));
    assert!(!res.is_user_event("input.t"));

    // Verify inverted_changes restores the original document
    let inverted = res.inverted_changes();
    let restored = inverted.apply(res.state_ref().doc().clone())?;
    assert_eq!(restored.to_string(), "hello world");
    Ok(())
}

#[test]
fn supports_slice_doc_and_read_only_facet() {
    let state = State::from("line1\nline2\nline3").with_extensions(Extension::from([
        LineSeparator("\r\n").into_ext(),
        ReadOnly(true).into_ext(),
    ]));

    assert!(state.read_only());
    assert_eq!(state.line_break(), "\r\n");
    assert_eq!(state.slice_doc(None, None), "line1\r\nline2\r\nline3");
    assert_eq!(state.slice_doc(Some(0), Some(5)), "line1");
}
