use std::time::Instant;
use texera_core::{AllowMultipleSelections, ChangeSet, ChangeSpec, Selection, State, Text, tr};

#[test]
fn bench_1000_cursors_apply() {
    let num_lines = 10_000;
    let num_cursors = 1_000;
    let line = "fn some_function_name_here() -> Result<(), Box<dyn Error>> {\n";
    let doc_str = line.repeat(num_lines);
    let doc = Text::from(doc_str.as_str());

    let mut specs = Vec::with_capacity(num_cursors);
    let line_len = line.chars().count();
    for i in 0..num_cursors {
        let pos = i * line_len + 5;
        specs.push(ChangeSpec::new(pos, Some(pos), Some("x".into())).unwrap());
    }

    let cs = ChangeSet::from_slice(&specs, doc.len_chars()).unwrap();

    let start = Instant::now();
    let new_doc = cs.apply(doc.clone()).unwrap();
    let duration = start.elapsed();
    println!("\n>>> apply 1000 cursors (10k lines doc) took: {:?} <<<\n", duration);
    assert_eq!(new_doc.len_chars(), doc.len_chars() + num_cursors);
    assert_eq!(new_doc.line(1).unwrap().text(), "fn soxme_function_name_here() -> Result<(), Box<dyn Error>> {\n");
}

#[test]
fn bench_1000_cursors_delete_apply() {
    let num_lines = 10_000;
    let num_cursors = 1_000;
    let line = "fn some_function_name_here() -> Result<(), Box<dyn Error>> {\n";
    let doc_str = line.repeat(num_lines);
    let doc = Text::from(doc_str.as_str());

    let mut specs = Vec::with_capacity(num_cursors);
    let line_len = line.chars().count();
    for i in 0..num_cursors {
        let pos = i * line_len + 5;
        specs.push(ChangeSpec::new(pos, Some(pos + 1), None).unwrap());
    }

    let cs = ChangeSet::from_slice(&specs, doc.len_chars()).unwrap();

    let start = Instant::now();
    let new_doc = cs.apply(doc.clone()).unwrap();
    let duration = start.elapsed();
    println!("\n>>> delete 1000 cursors (10k lines doc) took: {:?} <<<\n", duration);
    assert_eq!(new_doc.len_chars(), doc.len_chars() - num_cursors);
    assert_eq!(new_doc.line(1).unwrap().text(), "fn soe_function_name_here() -> Result<(), Box<dyn Error>> {\n");
}

#[test]
fn bench_1000_cursors_state_update() {
    let num_lines = 10_000;
    let num_cursors = 1_000;
    let line = "fn some_function_name_here() -> Result<(), Box<dyn Error>> {\n";
    let doc_str = line.repeat(num_lines);

    let line_len = line.chars().count();
    let cursors: Vec<_> = (0..num_cursors).map(|i| Selection::cursor(i * line_len + 5)).collect();
    let selection = Selection::from(cursors);

    let state = State::from(doc_str.as_str())
        .with_extensions(AllowMultipleSelections(true));
    let state = state.update(tr!({ selection: selection })).unwrap().into_state();

    let spec = state.replace_selection("x");
    let old_len = state.doc().len_chars();

    let start = Instant::now();
    let next_state = state.update(spec).unwrap().into_state();
    let duration = start.elapsed();
    println!("\n>>> state.update 1000 cursors took: {:?} <<<\n", duration);
    assert_eq!(next_state.doc().len_chars(), old_len + num_cursors);
    assert_eq!(next_state.doc().line(1).unwrap().text(), "fn soxme_function_name_here() -> Result<(), Box<dyn Error>> {\n");
}
