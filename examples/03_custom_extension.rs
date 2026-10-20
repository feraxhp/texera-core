//! # Example 3: Custom Extensions, StateFields, and Compartments
//!
//! Demonstrates:
//! 1. Defining a custom [`StateField`] that tracks document edits.
//! 2. Defining a custom [`Facet`] for multi-value configuration tags.
//! 3. Using a [`Compartment`] for runtime dynamic reconfiguration.
//! 4. Querying fields and facets from a [`State`].

use texera_core::{
    Compartment, Extension, Facet, IntoExtension, Multi, State, StateField, TabSize,
    Transaction, tr,
};

// 1. Define a custom StateField to track edit history counts
struct EditCounter;

impl StateField for EditCounter {
    type Value = usize;

    fn create(_state: &State) -> Self::Value {
        0
    }

    fn update(value: &Self::Value, tr: &Transaction) -> Self::Value {
        if tr.doc_changed() {
            value + 1
        } else {
            *value
        }
    }
}

// 2. Define a custom Facet that aggregates feature flags or tags
#[derive(Clone, Debug, PartialEq, Eq)]
struct FeatureTag(&'static str);

impl Facet for FeatureTag {
    type Mode = Multi;

    fn compare_input(a: &Self, b: &Self) -> bool {
        a == b
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 1. Initializing State with Custom Extensions ===");

    // Create a dynamic compartment for tab size so it can be changed at runtime
    let tab_comp = Compartment::new();

    let extensions = Extension::from([
        EditCounter::init(),
        tab_comp.of(TabSize(4)),
        FeatureTag("syntax_highlighting").into_ext(),
        FeatureTag("line_numbers").into_ext(),
    ]);

    let state = State::from("fn add(a: i32, b: i32) -> i32 {\n    a + b\n}")
        .with_extensions(extensions);

    println!("Initial TabSize: {}", state.tab_size());
    println!("Initial EditCount: {:?}", state.field::<EditCounter>());

    let tags = state.facet::<FeatureTag>();
    println!("Active feature tags ({}):", tags.len());
    for tag in tags {
        println!("  - {:?}", tag.0);
    }

    println!("\n=== 2. Advancing Edits and Tracking StateField Updates ===");
    // Make an edit
    let state = state.update(tr!({
        changes: { from: 0, insert: "// Math helper\n" },
    }))?.into_state();

    println!("Document length: {} chars", state.doc().len_chars());
    println!("EditCount after 1st edit: {:?}", state.field::<EditCounter>());

    // Make another edit
    let doc_len = state.doc().len_chars();
    let state = state.update(tr!({
        changes: { from: doc_len, insert: "\n// End of file" },
    }))?.into_state();

    println!("EditCount after 2nd edit: {:?}", state.field::<EditCounter>());
    assert_eq!(state.field::<EditCounter>(), Some(&2));

    println!("\n=== 3. Dynamic Reconfiguration via Compartment ===");
    // Reconfigure the tab size compartment from 4 to 2 in a transaction
    let state = state.update(tr!({
        effects: [tab_comp.reconfigure(TabSize(2))],
    }))?.into_state();

    println!("TabSize after reconfiguration: {}", state.tab_size());
    assert_eq!(state.tab_size(), 2);

    println!("\nExample 03 completed successfully!");
    Ok(())
}
