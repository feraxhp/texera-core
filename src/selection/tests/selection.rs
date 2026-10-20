use mynox::Report;
use crate::{Assoc, ChangeSet, ChangeSpec, Selection};


fn format_ranges<'a>(sel: &'a Selection) -> String {
    sel.ranges_ref()
        .iter()
        .map(| s | format!("{}-{} ", s.anchor(), s.head()) )
        .collect()
}

#[test]
fn stores_ranges_with_a_primary_range() -> Result<(), Report> {
    let sel = Selection::from_with_head([
        Selection::range(0, 1), 
        Selection::range(3, 2), 
        Selection::range(4, 5),
    ], 1)?;
    
    assert_eq!(sel.main().from(), 2);
    assert_eq!(sel.main().to(), 3);
    assert_eq!(sel.main().anchor(), 3);
    assert_eq!(sel.main().head(), 2);
    
    assert_eq!(&format_ranges(&sel), "0-1 3-2 4-5 ");
    Ok(())
}

#[test]
fn rejects_out_of_bounds_main_index() {
    let res_eq = Selection::from_with_head([
        Selection::range(0, 1),
        Selection::range(2, 3),
    ], 2);
    assert!(res_eq.is_err());

    let res_gt = Selection::from_with_head([
        Selection::range(0, 1),
    ], 5);
    assert!(res_gt.is_err());
}

#[test]
fn merges_and_sorts_ranges_when_normalizing() {
    let sel = Selection::from([
      Selection::range(10, 12),
      Selection::range(6, 7),
      Selection::range(4, 5),
      Selection::range(3, 4),
      Selection::range(0, 6),
      Selection::range(7, 8),
      Selection::range(9, 13),
      Selection::range(13, 14),
    ]);
    
    assert_eq!(&format_ranges(&sel), "0-6 6-7 7-8 9-13 13-14 ");
}

#[test]
fn merges_adjacent_point_ranges_when_normalizing() {
    let sel = Selection::from([
      Selection::range(10, 12),
      Selection::range(12, 12),
      Selection::range(12, 12),
      Selection::range(10, 10),
      Selection::range(8, 10),
    ]);

    assert_eq!(&format_ranges(&sel), "8-10 10-12 ");
}

#[test]
fn preserves_the_direction_of_the_last_range_when_merging_ranges() {
    let sel = Selection::from([
        Selection::range(0, 2), 
        Selection::range(10, 1)
    ]);
    
    assert_eq!(&format_ranges(&sel), "10-0 ");
}

#[test]
fn maps_non_empty_ranges_inwards_and_merges_collapsed_cursors() -> Result<(), Report> {
    // Non-empty range [2, 5]: inserting at 2 and at 5 should stay outside [2, 5] -> [3, 6]
    let range = Selection::range(5, 2); // inverted range from 2 to 5
    let cs = ChangeSet::from_slice(
        &[
            ChangeSpec::new(2, Some(2), Some("A".into()))?,
            ChangeSpec::new(5, Some(5), Some("B".into()))?,
        ],
        10,
    )?;
    let mapped_range = range.map(cs.description(), Assoc::After)?;
    assert_eq!(mapped_range.from(), 3);
    assert_eq!(mapped_range.to(), 6);
    assert_eq!(mapped_range.anchor(), 6);
    assert_eq!(mapped_range.head(), 3);

    // Two cursors at 2 and 5 should merge into a single cursor at 1 when 1..6 is deleted
    let sel = Selection::from_with_head([Selection::cursor(2), Selection::cursor(5)], 1)?;
    let del_cs = ChangeSet::from_slice(&[ChangeSpec::new(1, Some(6), None)?], 10)?;
    let mapped_sel = sel.map(del_cs.description(), Assoc::After)?;
    assert_eq!(mapped_sel.count(), 1);
    assert_eq!(mapped_sel.main().from(), 1);
    assert_eq!(mapped_sel.main_index(), 0);
    Ok(())
}

#[test]
fn add_range_and_replace_range_normalize_and_track_main() {
    let sel = Selection::from([Selection::range(0, 2), Selection::range(6, 8)]);
    let added = sel.add_range(Selection::range(3, 5), true);
    assert_eq!(&format_ranges(&added), "0-2 3-5 6-8 ");
    assert_eq!(added.main().from(), 3);
    assert_eq!(added.main_index(), 1);

    let replaced = added.replace_range(Selection::range(1, 7), 1);
    assert_eq!(&format_ranges(&replaced), "0-8 ");
    assert_eq!(replaced.count(), 1);
    assert_eq!(replaced.main_index(), 0);
}
