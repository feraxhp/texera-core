//! # Example 1: Basic Editing and Undo/Redo
//!
//! Demonstrates:
//! 1. Initializing an immutable [`State`] with a document.
//! 2. Reading document lines and character offsets.
//! 3. Applying declarative modifications via the [`tr!`] macro.
//! 4. Inverting changes for an undo/redo pipeline.

use texera_core::{Selection, State, tr};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 1. Initializing Editor State ===");
    let initial_text = "fn main() {\n    println!(\"Hello\");\n}";
    let state = State::from(initial_text);

    println!("Document length: {} chars", state.doc().len_chars());
    println!("Document lines: {}", state.doc().len_lines());

    // Inspect individual lines (0-based index)
    let line0 = state.doc().line(0)?;
    println!("Line 0: {:?} (range {}..{})", line0.text_trimmed(), line0.from(), line0.to());

    let line1 = state.doc().line(1)?;
    println!("Line 1: {:?} (range {}..{})", line1.text_trimmed(), line1.from(), line1.to());

    println!("\n=== 2. Applying Transactions with `tr!` ===");
    // In `fn main() {\n    println!("Hello");\n}`, "Hello" is at character range 26..31.
    // Replace "Hello" with "Hello, Texera!" and move cursor right after it.
    let result = state.update(tr!({
        changes: { from: 26, to: 31, insert: "Hello, Texera!" },
        selection: Selection::cursor(40),
        user_event: "input.type",
    }))?;

    assert!(result.doc_changed());
    assert!(result.is_user_event("input"));

    let next_state = result.state_ref();
    println!("Updated document:\n{}", next_state.doc());
    println!("Cursor position: {}", next_state.selection().main().head());

    println!("\n=== 3. Undo by Inverting Changes ===");
    // Invert the transaction changes to restore the original document
    let undo = result.inverted_changes();
    let restored_doc = undo.apply(next_state.doc().clone())?;

    println!("Restored document after Undo:\n{}", restored_doc);
    assert_eq!(restored_doc.to_string(), initial_text);

    println!("\nExample 01 completed successfully!");
    Ok(())
}
