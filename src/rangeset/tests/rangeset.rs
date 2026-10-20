use std::sync::Arc;

use crate::rangeset::RangeComparator;
use crate::rangeset::RangeSetUpdate;
use crate::rangeset::SpanIterator;
use crate::rangeset::RangeValue;
use crate::rangeset::RangeSet;
use crate::rangeset::Range;
use crate::ChangeDesc;
use crate::ChangeSet;
use crate::ChangeSpec;
use crate::Text;


#[derive(Clone, Debug, PartialEq, Eq)]
struct Value {
    start_side: i32,
    end_side: i32,
    point: bool,
    name: Option<String>,
    pos: Option<usize>,
}

impl Value {
    fn new(
        start_side: Option<i32>,
        end_side: Option<i32>,
        point: bool,
        name: Option<&str>,
        pos: Option<usize>,
        empty: bool,
    ) -> Self {
        Self {
            start_side: start_side.unwrap_or(1),
            end_side: end_side.unwrap_or(if empty { 1 } else { -1 }),
            point: empty || point,
            name: name.map(|s| s.to_string()),
            pos,
        }
    }

    fn names(v: &[Value]) -> String {
        let mut result = Vec::new();
        for val in v {
            if let Some(ref name) = val.name {
                result.push(name.as_str());
            } else if val.point {
                result.push("POINT");
            }
        }
        result.join("/")
    }
}

impl RangeValue for Value {
    fn eq(&self, other: &Self) -> bool { self.name == other.name }
    fn start_side(&self) -> i32 { self.start_side }
    fn end_side(&self) -> i32 { self.end_side }
    fn point(&self) -> bool { self.point }
}

fn mk(from: usize, to: usize) -> Range<Value> {
    Value::new(None, None, false, None, None, from == to).range(from, to)
}

fn mk_pt(pos: usize) -> Range<Value> {
    Value::new(None, None, false, None, None, true).range(pos, pos)
}

fn mk_named(from: usize, to: usize, name: &str) -> Range<Value> {
    Value::new(None, None, false, Some(name), None, from == to).range(from, to)
}

fn mk_pos(from: usize, to: usize, pos: usize) -> Range<Value> {
    Value::new(None, None, false, None, Some(pos), from == to).range(from, to)
}

fn mk_sides(from: usize, to: usize, start_side: i32, end_side: i32) -> Range<Value> {
    Value::new(
        Some(start_side),
        Some(end_side),
        false,
        None,
        None,
        from == to,
    )
    .range(from, to)
}

fn mk_point_spec(
    from: usize,
    to: usize,
    name: Option<&str>,
    start_side: Option<i32>,
    end_side: Option<i32>,
) -> Range<Value> {
    Value::new(start_side, end_side, true, name, None, from == to).range(from, to)
}

fn mk_set(ranges: Vec<Range<Value>>) -> RangeSet<Value> {
    RangeSet::of(ranges, false)
}

fn change_desc(changes: &[(usize, usize, usize)]) -> ChangeDesc {
    let mut specs = Vec::new();
    for &(from, to, len) in changes {
        if len > 0 {
            let text = Text::from("x".repeat(len).as_str());
            specs.push(ChangeSpec::new(from, Some(to), Some(text)).unwrap());
        }
        if from < to {
            specs.push(ChangeSpec::new(from, Some(to), None).unwrap());
        }
    }
    ChangeSet::from_slice(&specs, 5100)
        .unwrap()
        .description()
        .clone()
}

fn small_ranges() -> Vec<Range<Value>> {
    (0..5000)
        .map(|i| mk_pos(i, i + 1 + (i % 4), i))
        .collect()
}

fn set0() -> RangeSet<Value> {
    mk_set(small_ranges())
}

fn check_set(set: &RangeSet<Value>) {
    let mut count = 0;
    set.between(0, set.length(), |from, _, value| {
        count += 1;
        if let Some(pos) = value.pos {
            assert_eq!(from, pos);
        }
        true
    });
    assert_eq!(count, set.size());
}

#[test]
fn divides_a_set_into_chunks_and_layers() {
    let mut ranges = small_ranges();
    ranges.push(mk_pos(1000, 4000, 1000));
    ranges.push(mk_pos(2000, 3000, 2000));
    ranges.sort_by_key(|r| r.from);
    let set = mk_set(ranges);
    assert_eq!(set.size(), 5002);
    assert!(set.chunk.len() > 1);
    assert!(set.next_layer.as_ref().unwrap().size() > 0);
    check_set(&set);
}

#[test]
fn complains_about_misordered_ranges() {
    assert!(RangeSet::try_of(vec![mk(8, 9), mk(7, 10)], false).is_err());
    assert!(
        RangeSet::try_of(
            vec![mk_sides(1, 1, 1, 1), mk_sides(1, 1, -1, -1)],
            false
        )
        .is_err()
    );
}

#[test]
fn update_can_add_ranges() {
    let s0 = set0();
    let set = s0.update(RangeSetUpdate {
        add: vec![mk_pos(4000, 4000, 4000)],
        ..Default::default()
    });
    assert_eq!(set.size(), 5001);
    assert!(Arc::ptr_eq(&set.chunk[0], &s0.chunk[0]));
}

#[test]
fn update_can_add_a_large_amount_of_ranges() {
    let s0 = set0();
    let ranges: Vec<_> = (0..4000).step_by(2).map(mk_pt).collect();
    let set = s0.update(RangeSetUpdate {
        add: ranges,
        ..Default::default()
    });
    assert_eq!(set.size(), 2000 + s0.size());
    check_set(&set);
}

#[test]
fn update_can_filter_ranges() {
    let s0 = set0();
    let set = s0.update(RangeSetUpdate {
        filter: Some(Box::new(|from, _, _| from >= 2500)),
        ..Default::default()
    });
    assert_eq!(set.size(), 2500);
    assert!(set.chunk.len() < s0.chunk.len());
    check_set(&set);
}

#[test]
fn update_can_filter_all_over() {
    let s0 = set0();
    let set = s0.update(RangeSetUpdate {
        filter: Some(Box::new(|from, _, _| (from % 200) >= 100)),
        ..Default::default()
    });
    assert_eq!(set.size(), 2500);
    check_set(&set);
}

#[test]
fn update_collapses_the_chunks_when_removing_almost_all_ranges() {
    let s0 = set0();
    let set = s0.update(RangeSetUpdate {
        filter: Some(Box::new(|from, _, _| from == 500 || from == 501)),
        ..Default::default()
    });
    assert_eq!(set.size(), 2);
    assert_eq!(set.chunk.len(), 1);
}

#[test]
fn update_calls_filter_on_precisely_those_ranges_touching_the_filter_range() {
    let ranges: Vec<_> = (0..1000).map(|i| mk_pos(i, i + 1, i)).collect();
    let set = mk_set(ranges);
    let called = std::cell::RefCell::new(Vec::new());
    set.update(RangeSetUpdate {
        filter: Some(Box::new(|from, to, _| {
            called.borrow_mut().push((from, to));
            true
        })),
        filter_from: Some(400),
        filter_to: Some(600),
        ..Default::default()
    });
    let called = called.into_inner();
    assert_eq!(called.len(), 202);
    for (j, i) in (399..=600).enumerate() {
        assert_eq!(called[j], (i, i + 1));
    }
}

#[test]
fn update_returns_the_empty_set_when_filter_removes_everything() {
    let set = set0().update(RangeSetUpdate {
        filter: Some(Box::new(|_, _, _| false)),
        ..Default::default()
    });
    assert_eq!(set, RangeSet::empty());
}

fn test_map(
    positions: Vec<Range<Value>>,
    changes: &[(usize, usize, usize)],
    expected: &[(usize, usize)],
) {
    let set = mk_set(positions);
    let mapped = set.map(&change_desc(changes));
    let out: Vec<_> = mapped.iter().map(|r| (r.from, r.to)).collect();
    assert_eq!(out, expected);
}

#[test]
fn map_can_map_through_changes() {
    test_map(
        vec![mk_pt(1), mk_pt(4), mk_pt(10)],
        &[(0, 0, 1), (2, 3, 0), (8, 8, 20)],
        &[(2, 2), (4, 4), (30, 30)],
    );
}

#[test]
fn map_takes_inclusivity_into_account() {
    test_map(
        vec![mk_sides(1, 2, -1, 1)],
        &[(1, 1, 2), (2, 2, 2)],
        &[(1, 6)],
    );
}

#[test]
fn map_uses_side_to_determine_mapping_of_points() {
    test_map(
        vec![mk_sides(1, 1, -1, -1), mk_sides(1, 1, 1, 1)],
        &[(1, 1, 2)],
        &[(1, 1), (3, 3)],
    );
}

#[test]
fn map_defaults_to_exclusive_on_both_sides() {
    test_map(
        vec![mk(1, 2)],
        &[(1, 1, 2), (4, 4, 2)],
        &[(3, 4)],
    );
}

#[test]
fn map_drops_point_ranges() {
    test_map(vec![mk(1, 2)], &[(1, 2, 0), (1, 1, 1)], &[]);
}

#[test]
fn map_drops_ranges_in_deleted_regions() {
    test_map(vec![mk(1, 2), mk_pt(3)], &[(0, 4, 0)], &[]);
}

#[test]
fn map_shrinks_ranges() {
    test_map(
        vec![mk(2, 4), mk(2, 8), mk(6, 8)],
        &[(3, 7, 0)],
        &[(2, 3), (2, 4), (3, 4)],
    );
}

#[test]
fn map_leaves_point_ranges_on_change_boundaries() {
    test_map(vec![mk_pt(2), mk_pt(4)], &[(2, 4, 6)], &[(2, 2), (8, 8)]);
}

#[test]
fn map_can_collapse_chunks() {
    let s0 = set0();
    let smaller = s0.map(&change_desc(&[(30, 4500, 0)]));
    assert!(smaller.chunk.len() < s0.chunk.len());
    let empty = smaller.map(&change_desc(&[(0, 1000, 0)]));
    assert_eq!(empty, RangeSet::empty());
}

#[derive(Default)]
struct Comparator {
    ranges: Vec<usize>,
}

impl Comparator {
    fn add_range(&mut self, from: usize, to: usize) {
        if let Some(last) = self.ranges.last_mut()
            && *last == from
        {
            *last = to;
        } else {
            self.ranges.push(from);
            self.ranges.push(to);
        }
    }
}

impl RangeComparator<Value> for Comparator {
    fn compare_range(&mut self, from: usize, to: usize, _: &[Value], _: &[Value]) {
        self.add_range(from, to);
    }

    fn compare_point(&mut self, from: usize, to: usize, _: Option<&Value>, _: Option<&Value>) {
        self.add_range(from, to);
    }
}

fn test_compare(
    set: RangeSet<Value>,
    changes: &[(usize, usize, usize)],
    add: Vec<Range<Value>>,
    filter: Option<Box<dyn Fn(usize, usize, &Value) -> bool>>,
    expected: &[usize],
) {
    let doc_changes = change_desc(changes);
    let mut new_set = if !changes.is_empty() {
        set.map(&doc_changes)
    } else {
        set.clone()
    };
    if !add.is_empty() || filter.is_some() {
        new_set = new_set.update(RangeSetUpdate {
            add,
            filter,
            ..Default::default()
        });
    }
    let mut comp = Comparator::default();
    RangeSet::compare(&[&set], &[&new_set], &doc_changes, &mut comp, None);
    assert_eq!(comp.ranges, expected);
}

#[test]
fn compare_notices_added_ranges() {
    test_compare(
        mk_set(vec![mk_named(2, 4, "a"), mk_named(8, 11, "a")]),
        &[],
        vec![mk_named(3, 9, "b"), mk_named(106, 107, "b")],
        None,
        &[3, 9, 106, 107],
    );
}

#[test]
fn compare_notices_deleted_ranges() {
    test_compare(
        mk_set(vec![
            mk_named(4, 6, "a"),
            mk_named(5, 7, "b"),
            mk_named(6, 8, "c"),
            mk_named(20, 30, "d"),
        ]),
        &[],
        vec![],
        Some(Box::new(|from, _, _| from != 5 && from != 20)),
        &[5, 7, 20, 30],
    );
}

#[test]
fn compare_recognizes_identical_ranges() {
    test_compare(
        mk_set(vec![mk_named(0, 50, "a")]),
        &[],
        vec![mk_named(10, 40, "a")],
        Some(Box::new(|_, _, _| false)),
        &[0, 10, 40, 50],
    );
}

#[test]
fn compare_skips_changes() {
    test_compare(
        mk_set(vec![mk_named(0, 20, "a")]),
        &[(5, 15, 20)],
        vec![],
        Some(Box::new(|_, _, _| false)),
        &[0, 5, 25, 30],
    );
}

#[test]
fn compare_ignores_identical_sub_nodes() {
    let ranges: Vec<_> = (0..1000).map(|i| mk_named(i, i + 1, "a")).collect();
    test_compare(
        mk_set(ranges),
        &[(900, 1000, 0)],
        vec![mk_named(850, 860, "b")],
        None,
        &[850, 860],
    );
}

#[test]
fn compare_ignores_changes_in_points() {
    let mut ranges = vec![mk_point_spec(3, 997, None, None, None)];
    for i in (0..1000).step_by(2) {
        ranges.push(mk_named(i, i + 1, "a"));
    }
    ranges.sort_by_key(|r| r.from);
    test_compare(mk_set(ranges), &[(300, 500, 100)], vec![], None, &[]);
}

#[test]
fn compare_notices_adding_a_point() {
    test_compare(
        mk_set(vec![mk_point_spec(3, 50, None, None, None)]),
        &[],
        vec![mk_point_spec(40, 80, None, None, None)],
        None,
        &[50, 80],
    );
}

#[test]
fn compare_notices_removing_a_point() {
    test_compare(
        mk_set(vec![mk_point_spec(3, 50, None, None, None)]),
        &[],
        vec![],
        Some(Box::new(|_, _, _| false)),
        &[3, 50],
    );
}

#[test]
fn compare_can_handle_multiple_changes() {
    let mut ranges = Vec::new();
    for i in (0..200).step_by(2) {
        let end = i + 1 + (i + 49) / 50;
        let name = format!("{i}-{end}");
        ranges.push(mk_named(i, end, &name));
    }
    test_compare(
        mk_set(ranges),
        &[(0, 0, 50), (50, 100, 0), (150, 200, 0)],
        vec![],
        Some(Box::new(|from, _, _| from % 50 > 0)),
        &[50, 51, 100, 103, 148, 153],
    );
}

#[test]
fn compare_reports_point_decorations_with_different_cover() {
    test_compare(
        mk_set(vec![
            mk_sides(0, 4, 1, -1),
            mk_point_spec(1, 3, None, Some(-1), Some(1)),
        ]),
        &[(2, 4, 0)],
        vec![],
        None,
        &[1, 2],
    );
}

#[derive(Default)]
struct SpanBuilder {
    spans: Vec<String>,
}

impl SpanIterator<Value> for SpanBuilder {
    fn span(&mut self, from: usize, to: usize, active: &[Value], _open_start: usize) {
        let name = Value::names(active);
        let len = to - from;
        if name.is_empty() {
            self.spans.push(format!("{len}"));
        } else {
            self.spans.push(format!("{len}={name}"));
        }
    }

    fn point(
        &mut self,
        from: usize,
        to: usize,
        value: &Value,
        _active: &[Value],
        _open_start: usize,
        _index: usize,
    ) {
        let prefix = if to > from {
            format!("{}=", to - from)
        } else {
            String::new()
        };
        let label = match &value.name {
            Some(name) => format!("[{name}]"),
            None => "ø".to_string(),
        };
        self.spans.push(format!("{prefix}{label}"));
    }
}

fn test_spans(sets: &[&RangeSet<Value>], start: usize, end: usize, expected: &str) {
    let mut builder = SpanBuilder::default();
    RangeSet::spans(sets, start, end, &mut builder, None);
    assert_eq!(builder.spans.join(" "), expected);
}

#[test]
fn spans_separates_the_range_in_covering_spans() {
    let set = mk_set(vec![
        mk_named(3, 8, "one"),
        mk_named(5, 8, "two"),
        mk_named(10, 12, "three"),
    ]);
    test_spans(&[&set], 0, 15, "3 2=one 3=two/one 2 2=three 3");
}

#[test]
fn spans_can_retrieve_a_limited_range() {
    let mut decos = vec![mk_named(0, 200, "wide")];
    for i in 0..100 {
        let name = format!("span{i}");
        decos.push(mk_named(i * 2, i * 2 + 2, &name));
    }
    let set = mk_set(decos);
    let start = 20;
    let end = start + 6;
    test_spans(&[&set], start, end, "2=span10/wide 2=span11/wide 2=span12/wide");
}

#[test]
fn spans_reads_from_multiple_sets_at_once() {
    let one = mk_set(vec![
        mk_named(2, 3, "x"),
        mk_named(5, 10, "y"),
        mk_named(10, 12, "z"),
    ]);
    let two = mk_set(vec![mk_named(0, 6, "a"), mk_named(10, 12, "b")]);
    test_spans(&[&one, &two], 0, 12, "2=a 1=x/a 2=a 1=y/a 4=y 2=z/b");
}

#[test]
fn spans_orders_active_ranges_by_origin_set() {
    let one = mk_set(vec![mk_named(2, 10, "a"), mk_named(20, 30, "a")]);
    let two = mk_set(vec![
        mk_named(3, 4, "b"),
        mk_named(8, 12, "b"),
        mk_named(18, 22, "b"),
    ]);
    let three = mk_set(vec![mk_named(0, 25, "c")]);
    test_spans(
        &[&one, &two, &three],
        0,
        25,
        "2=c 1=a/c 1=a/b/c 4=a/c 2=a/b/c 2=b/c 6=c 2=b/c 2=a/b/c 3=a/c",
    );
}

#[test]
fn spans_doesnt_get_confused_by_same_place_points() {
    let set = mk_set(vec![
        mk_named(1, 1, "a"),
        mk_named(1, 1, "b"),
        mk_named(1, 1, "c"),
    ]);
    test_spans(&[&set], 0, 2, "1 [a] [b] [c] 1");
}

#[test]
fn spans_properly_resyncs_active_ranges_after_points() {
    let set = mk_set(vec![
        mk_named(0, 20, "r1"),
        mk_named(1, 10, "r2"),
        mk_point_spec(3, 12, Some("p"), None, None),
        mk_named(4, 8, "r3"),
        mk_named(5, 20, "r4"),
    ]);
    test_spans(&[&set], 0, 20, "1=r1 2=r2/r1 9=[p] 8=r4/r1");
}

#[test]
fn spans_doesnt_split_spans_on_ignored_ranges() {
    struct Recorder(Vec<usize>);
    impl SpanIterator<Value> for Recorder {
        fn span(&mut self, from: usize, to: usize, _: &[Value], _: usize) {
            self.0.push(from);
            self.0.push(to);
        }
        fn point(&mut self, from: usize, to: usize, _: &Value, _: &[Value], _: usize, _: usize) {
            self.0.push(from);
            self.0.push(to);
        }
    }
    let set = mk_set(vec![
        mk_named(0, 10, "a"),
        mk_point_spec(20, 30, Some("b"), None, None),
    ]);
    let mut rec = Recorder(Vec::new());
    RangeSet::spans(&[&set], 0, 30, &mut rec, Some(0));
    assert_eq!(rec.0, vec![0, 20, 20, 30]);
}

#[test]
fn spans_omits_points_that_are_covered_by_the_previous_point() {
    struct PointCounter(usize);
    impl SpanIterator<Value> for PointCounter {
        fn span(&mut self, _: usize, _: usize, _: &[Value], _: usize) {}
        fn point(&mut self, _: usize, _: usize, _: &Value, _: &[Value], _: usize, _: usize) {
            self.0 += 1;
        }
    }
    let set = mk_set(vec![
        mk_point_spec(0, 4, None, None, None),
        mk_named(1, 5, "a"),
        mk_point_spec(2, 4, None, None, None),
    ]);
    let mut counter = PointCounter(0);
    RangeSet::spans(&[&set], 0, 10, &mut counter, None);
    assert_eq!(counter.0, 1);
}

#[test]
fn spans_puts_smaller_spans_inside_bigger_ones_with_the_same_rank() {
    let set = mk_set(vec![
        mk_named(0, 3, "x"),
        mk_named(0, 1, "a"),
        mk_named(1, 2, "b"),
        mk_named(2, 3, "c"),
    ]);
    test_spans(&[&set], 0, 3, "1=a/x 1=b/x 1=c/x");
}

#[test]
fn iter_iterates_over_ranges() {
    let mut ranges = small_ranges();
    ranges.push(mk_pos(1000, 4000, 1000));
    ranges.push(mk_pos(2000, 3000, 2000));
    ranges.sort_by_key(|r| r.from);
    let set = mk_set(ranges);
    let mut count = 0;
    for r in set.iter() {
        let expected = if count > 2001 {
            count - 2
        } else if count > 1000 {
            count - 1
        } else {
            count
        };
        assert_eq!(r.from, expected);
        count += 1;
    }
    assert_eq!(count, 5002);
}

#[test]
fn iter_can_iterate_over_a_subset() {
    let s0 = set0();
    let mut count = 0;
    for r in s0.iter_from(1000) {
        if r.from > 2000 {
            break;
        }
        assert_eq!(r.to, r.from + 1 + (r.from % 4));
        count += 1;
    }
    assert_eq!(count, 1003);
}

#[test]
fn between_iterates_over_ranges() {
    let mut found = 0;
    set0().between(100, 200, |from, to, _| {
        assert_eq!(to, from + 1 + (from % 4));
        assert!(to >= 100);
        assert!(from <= 200);
        found += 1;
        true
    });
    assert_eq!(found, 103);
}

#[test]
fn between_returns_ranges_in_a_zero_length_set() {
    let set = RangeSet::of(vec![mk(0, 0)], false);
    let mut found = Vec::new();
    set.between(0, 0, |from, to, _| {
        found.push((from, to));
        true
    });
    assert_eq!(found, vec![(0, 0)]);
}
