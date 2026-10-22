use std::time::Instant;
use texera_core::{
    count_column, find_cluster_break, AllowMultipleSelections, Assoc, ChangeSet, ChangeSpec,
    LineSeparator, Selection, SelectionRange, State, Text,
};

#[test]
fn bench_monotonic_selection_map_1000_cursors() {
    let num_cursors = 1_000;
    let step = 100;
    let total_len = num_cursors * step;

    let cursors: Vec<_> = (0..num_cursors)
        .map(|i| SelectionRange::cursor(i * step + 50))
        .collect();
    let selection = Selection::from(cursors);

    let mut specs = Vec::with_capacity(num_cursors);
    for i in 0..num_cursors {
        let pos = i * step + 10;
        specs.push(ChangeSpec::new(pos, Some(pos), Some("inserted_text".into())).unwrap());
    }
    let cs = ChangeSet::from_slice(&specs, total_len).unwrap();
    let desc = cs.description();

    let start = Instant::now();
    let mapped = selection.map(desc, Assoc::After).unwrap();
    let duration = start.elapsed();

    println!("\n>>> monotonic selection.map (1000 cursors through 1000 edits) took: {:?} <<<", duration);

    assert_eq!(mapped.count(), num_cursors);
    let insert_len = "inserted_text".chars().count();
    for (i, r) in mapped.ranges_ref().iter().enumerate() {
        let expected = i * step + 50 + (i + 1) * insert_len;
        assert_eq!(r.from(), expected);
    }
}

#[test]
fn bench_fast_path_single_cursor_merge() {
    let iterations = 100_000;
    let start = Instant::now();
    for i in 0..iterations {
        let sel = Selection::from([SelectionRange::cursor(i)]);
        assert_eq!(sel.main().from(), i);
    }
    let duration = start.elapsed();
    println!(">>> Selection::from single cursor (100k iterations) took: {:?} <<<", duration);
}

#[test]
fn bench_state_iter_selections_no_alloc() {
    let num_cursors = 10_000;
    let cursors: Vec<_> = (0..num_cursors).map(SelectionRange::cursor).collect();
    let selection = Selection::from(cursors);
    let doc_str = "a".repeat(num_cursors);
    let state = State::with_selection(doc_str.as_str(), selection)
        .with_extensions(AllowMultipleSelections(true));

    let start = Instant::now();
    let mut count = 0;
    for (r, _is_main) in state.iter_selections() {
        count += r.from();
    }
    let duration = start.elapsed();
    println!(">>> state.iter_selections (10k cursors) took: {:?} (sum: {}) <<<", duration, count);
    assert_eq!(count, (num_cursors * (num_cursors - 1)) / 2);
}

#[test]
fn bench_word_at_ascii_fast_path() {
    let line_str = "fn calculate_total_amount_for_customer(customer_id: usize, order_count: usize) -> u64 {\n";
    let doc_str = line_str.repeat(1_000);
    let state = State::from(doc_str.as_str());

    let line_len = line_str.len();
    let iterations = 10_000;

    let start = Instant::now();
    for i in 0..iterations {
        let pos = (i % 500) * line_len + 15; // inside "calculate_total_amount_for_customer"
        let word = state.word_at(pos).unwrap();
        assert_eq!(state.slice_doc(Some(word.from()), Some(word.to())), "calculate_total_amount_for_customer");
    }
    let duration = start.elapsed();
    println!(">>> State::word_at ASCII fast path (10k lookups) took: {:?} <<<", duration);
}

#[test]
fn bench_slice_string_single_pass_normalization() {
    let line = "alpha\r\nbeta\r\ngamma\r\ndelta\r\n";
    let doc_str = line.repeat(2_000);
    let text = Text::from(doc_str.as_str());

    let start = Instant::now();
    let normalized = text.slice_string(0, None, Some("\n".to_string()));
    let duration = start.elapsed();

    println!(">>> Text::slice_string CRLF normalization (2k repetitions) took: {:?} <<<", duration);
    assert!(!normalized.contains("\r\n"));
    assert_eq!(normalized.matches('\n').count(), 8_000);
}

#[test]
fn bench_ascii_unicode_helpers() {
    let ascii_text = "const DEFAULT_BUFFER_CAPACITY: usize = 4096;";
    let iterations = 100_000;

    let start = Instant::now();
    for _ in 0..iterations {
        let col = count_column(ascii_text, 4);
        assert_eq!(col, ascii_text.len());
    }
    let dur_col = start.elapsed();
    println!(">>> count_column on ASCII (100k calls) took: {:?} <<<", dur_col);

    let start = Instant::now();
    for i in 0..ascii_text.len() {
        let next = find_cluster_break(ascii_text, i, true, true);
        assert_eq!(next, i + 1);
    }
    let dur_cluster = start.elapsed();
    println!(">>> find_cluster_break across ASCII took: {:?} <<<", dur_cluster);
}

#[test]
fn test_to_text_preserves_binary_zeroes() {
    let state = State::from("").with_extensions(LineSeparator("\r\n"));
    let input = "hello\x00world\r\nsecond\x00line";
    let text = state.to_text(input);
    assert_eq!(text.len_lines(), 2);
    assert_eq!(text.to_string(), "hello\x00world\nsecond\x00line");
}
