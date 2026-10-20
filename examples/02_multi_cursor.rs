//! # Example 2: Multi-Cursor Selections and Batch Edits
//!
//! Demonstrates:
//! 1. Enabling multi-cursor support with [`AllowMultipleSelections`].
//! 2. Creating multiple directional [`SelectionRange`]s and automatic overlap merging.
//! 3. Batch editing at each cursor using [`State::replace_selection`].
//! 4. Per-selection functional transformations using [`State::change_by_range`].

use texera_core::{AllowMultipleSelections, Selection, SelectionRange, State, tr};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 1. Setting Up Multi-Cursor State ===");
    let text = "apple\nbanana\ncherry\ndate";

    // Initialize state with multi-selection enabled
    let state = State::from(text).with_extensions(AllowMultipleSelections(true));

    // Place a cursor at the start of each line
    // "apple\n" (0..6), "banana\n" (6..13), "cherry\n" (13..20), "date" (20..24)
    let multi_selection = Selection::from([
        Selection::cursor(0),
        Selection::cursor(6),
        Selection::cursor(13),
        Selection::cursor(20),
    ]);

    let state = state.update(tr!({ selection: multi_selection }))?.into_state();
    println!("Active cursors: {}", state.selection().count());

    println!("\n=== 2. Batch Insertion via `replace_selection` ===");
    // Prepend "- " to each line by replacing the empty cursor selections
    let spec = state.replace_selection("- ");
    let state = state.update(spec)?.into_state();

    println!("Document after prepending bullets:\n{}", state.doc());

    println!("=== 3. Custom Transformations via `change_by_range` ===");
    // Now select the fruit names and transform them to uppercase
    // With "- " prepended (2 chars per line):
    // Line 0: "- apple\n"   fruit at 2..7
    // Line 1: "- banana\n"  fruit at 10..16
    // Line 2: "- cherry\n"  fruit at 19..25
    // Line 3: "- date"      fruit at 28..32
    let select_fruits = Selection::from([
        SelectionRange::range(2, 7),
        SelectionRange::range(10, 16),
        SelectionRange::range(19, 25),
        SelectionRange::range(28, 32),
    ]);

    let state = state.update(tr!({ selection: select_fruits }))?.into_state();

    // Use `change_by_range` to apply a dynamic change to each selected range
    let spec = state.change_by_range(|(range, _is_main)| {
        let text_slice = state.slice_doc(Some(range.from()), Some(range.to()));
        let uppercased = text_slice.to_uppercase();
        let len = uppercased.chars().count();
        Ok(tr!({
            changes: { from: range.from(), to: range.to(), insert: uppercased.as_str() },
            selection: SelectionRange::range(range.from(), range.from() + len),
        }))
    })?;

    let state = state.update(spec)?.into_state();
    println!("Document after uppercase transformation:\n{}", state.doc());

    println!("=== 4. Automatic Overlap Normalization ===");
    // If ranges overlap or touch, `Selection` automatically normalizes and merges them
    let overlapping = Selection::from([
        SelectionRange::range(0, 5),
        SelectionRange::range(3, 8),
        SelectionRange::range(12, 15),
    ]);

    println!(
        "Requested 3 ranges, normalized into {} non-overlapping ranges:",
        overlapping.count()
    );
    for (i, (r, is_main)) in overlapping.iter().enumerate() {
        println!("  Range {}: {}..{} (is_main: {})", i, r.from(), r.to(), is_main);
    }

    println!("\nExample 02 completed successfully!");
    Ok(())
}
