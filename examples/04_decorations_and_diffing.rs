//! # Example 4: RangeSets, Document Decorations, and Fast Diffing
//!
//! Demonstrates:
//! 1. Creating custom [`RangeValue`] items for syntax decorations and markers.
//! 2. Building and mapping a [`RangeSet`] across document text edits.
//! 3. Fast $O(1)$-skipping diffing between decoration sets via [`RangeSet::compare`].
//! 4. Rendering uniform document spans via [`RangeSet::spans`] and [`SpanIterator`].

use texera_core::{
    ChangeSet, ChangeSpec, MapMode, RangeComparator, RangeSet, RangeSetBuilder, RangeValue,
    SpanIterator,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Style(&'static str);

impl RangeValue for Style {
    fn eq(&self, other: &Self) -> bool {
        self == other
    }

    fn map_mode(&self) -> MapMode {
        MapMode::TrackDel
    }
}

// Visitor to collect differences between two RangeSets across an edit
#[derive(Default)]
struct DiffCollector {
    changed_ranges: Vec<(usize, usize, Vec<String>, Vec<String>)>,
}

impl RangeComparator<Style> for DiffCollector {
    fn compare_range(&mut self, from: usize, to: usize, active_a: &[Style], active_b: &[Style]) {
        self.changed_ranges.push((
            from,
            to,
            active_a.iter().map(|s| s.0.to_string()).collect(),
            active_b.iter().map(|s| s.0.to_string()).collect(),
        ));
    }

    fn compare_point(
        &mut self,
        _from: usize,
        _to: usize,
        _point_a: Option<&Style>,
        _point_b: Option<&Style>,
    ) {
    }
}

// Visitor to collect contiguous rendering spans
#[derive(Default)]
struct SpanCollector {
    spans: Vec<(usize, usize, Vec<&'static str>)>,
}

impl SpanIterator<Style> for SpanCollector {
    fn span(&mut self, from: usize, to: usize, active: &[Style], _open_start: usize) {
        self.spans.push((from, to, active.iter().map(|s| s.0).collect()));
    }

    fn point(
        &mut self,
        _from: usize,
        _to: usize,
        _value: &Style,
        _active: &[Style],
        _open: usize,
        _idx: usize,
    ) {
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 1. Building a RangeSet of Decorations ===");
    let mut builder = RangeSetBuilder::new();
    // Suppose doc is: "let answer = 42;"
    // Decorate "let" as keyword (0..3), "answer" as variable (4..10), "42" as number (13..15)
    builder.add(0, 3, Style("keyword"));
    builder.add(4, 10, Style("variable"));
    builder.add(13, 15, Style("number"));
    let old_decorations = builder.finish();

    println!("Total decorated ranges: {}", old_decorations.size());
    for r in old_decorations.iter() {
        println!("  Decoration: {}..{} => {:?}", r.from, r.to, r.value.0);
    }

    println!("\n=== 2. Mapping Decorations Across Document Changes ===");
    // Insert "pub " at index 0 (length increases by 4)
    let change = ChangeSet::from_slice(
        &[ChangeSpec::new(0, Some(0), Some("pub ".into()))?],
        16, // original document length
    )?;

    // Map old decorations over the edit
    let mapped_decorations = old_decorations.map(change.description());

    println!("Mapped decorations after inserting 'pub ':");
    for r in mapped_decorations.iter() {
        println!("  Shifted decoration: {}..{} => {:?}", r.from, r.to, r.value.0);
    }

    println!("\n=== 3. Diffing RangeSets with `compare` ===");
    // In the new version of decorations:
    // - "pub" (0..3) is added as keyword
    // - "let" (4..7) remains keyword
    // - "answer" (8..14) is changed to "constant"
    // - "42" (17..19) remains number
    let mut new_builder = RangeSetBuilder::new();
    new_builder.add(0, 3, Style("keyword"));
    new_builder.add(4, 7, Style("keyword"));
    new_builder.add(8, 14, Style("constant")); // changed from variable to constant
    new_builder.add(17, 19, Style("number"));
    let new_decorations = new_builder.finish();

    let mut diff = DiffCollector::default();
    RangeSet::compare(
        &[&old_decorations],
        &[&new_decorations],
        change.description(),
        &mut diff,
        None,
    );

    println!("Detected decoration changes in unchanged text gaps ({} regions):", diff.changed_ranges.len());
    for (from, to, old_styles, new_styles) in &diff.changed_ranges {
        println!("  At {}..{}: old={:?} -> new={:?}", from, to, old_styles, new_styles);
    }

    println!("\n=== 4. Iterating Spans for Rendering ===");
    let mut span_collector = SpanCollector::default();
    RangeSet::spans(&[&new_decorations], 0, 20, &mut span_collector, None);

    println!("Rendered document spans (0..20):");
    for (from, to, styles) in &span_collector.spans {
        let label = if styles.is_empty() { "plain".to_string() } else { styles.join(", ") };
        println!("  Span {:2}..{:2}: [{}]", from, to, label);
    }

    println!("\nExample 04 completed successfully!");
    Ok(())
}
