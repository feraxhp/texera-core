use mynox::Report;

use crate::selection::Assoc;
use crate::changes::ChangeDesc;
use crate::changes::Section;
use crate::changes::MapMode;
use crate::sec;


struct Case { pos: usize, exp: Option<usize>, assoc: Assoc, mode: MapMode }

macro_rules! c {
    ($pos:expr $(, $exp:expr )? $(,)?) => { Case { 
        pos: $pos, 
        exp: None $( .or(Some($exp)) )?, 
        assoc: Assoc::Before, 
        mode: MapMode::Simple 
    } };
    ($pos:expr, $($exp:expr )?, A ) => { Case { 
        pos: $pos, 
        exp: None $( .or(Some($exp)) )?,
        assoc: Assoc::Before, 
        mode: MapMode::TrackAfter,
    } };
    ($pos:expr, $($exp:expr )?, B ) => { Case { 
        pos: $pos, 
        exp: None $( .or(Some($exp)) )?,
        assoc: Assoc::Before, 
        mode: MapMode::TrackBefore,
    } };
    ($pos:expr, $($exp:expr )?, D ) => { Case { 
        pos: $pos, 
        exp: None $( .or(Some($exp)) )?,
        assoc: Assoc::Before, 
        mode: MapMode::TrackDel,
    } };
    ($pos:expr, $( $exp:expr )?, $assoc:expr ) => { Case { 
        pos: $pos, 
        exp: None $( .or(Some($exp)) )?,
        assoc: if ( $assoc < 0 ) { Assoc::Before } else { Assoc::After }, 
        mode: MapMode::Simple 
    } };
}

fn assert_postions(chs: Vec<Section>, cases: Vec<Case>) -> Result<(), Report> {
    let change = ChangeDesc { sections: chs };
    for case in cases {
        let pos = change.map_pos(case.pos, case.assoc, case.mode)?;
        assert_eq!(pos, case.exp);
    }
    Ok(())
}

#[test]
fn maps_through_an_insertion() -> Result<(), Report> {
    assert_postions(sec!(4; 0,2; 4), vec![ 
        c![0, 0], c![4, 4], c![4, 6, 1], 
        c![5, 7], c![8, 10] 
    ])
    ?; Ok(())
}

#[test]
fn maps_through_deletion() -> Result<(), Report> {
    assert_postions(sec!(4; 4,0; 4), vec![
        c![0, 0],
        c![4, 4], c![4, 4, D], c![4, 4, B], c![4,  , A],
        c![5, 4], c![5,  , D], c![5,  , B], c![5,  , A],
        c![7, 4],
        c![8, 4], c![8, 4, D], c![8,  , B], c![8, 4, A],
        c![9, 5], c![12, 8]
    ])
    ?; Ok(())
}

#[test]
fn maps_through_multiple_insertions() -> Result<(), Report> {
    assert_postions(sec!(0,2; 2; 0,2; 2; 0,2), vec![
        c![0, 0], c![0, 2, 1], c![1, 3], 
        c![2, 4], c![2, 6, 1], c![3, 7], 
        c![4, 8], c![4, 10, 1]
    ])
    ?; Ok(())
}

#[test]
fn maps_through_multiple_deletions() -> Result<(), Report> {
    assert_postions(sec!(2,0; 2; 2,0; 2; 2,0), vec![
        c![0, 0], c![1, 0], c![2, 0], 
        c![3, 1], c![4, 2], c![5, 2], 
        c![6, 2], c![7, 3], c![8, 4], 
        c![9, 4], c![10, 4]
    ])
    ?; Ok(())
}

#[test]
fn maps_through_mixed_edits() -> Result<(), Report> {
    assert_postions(sec!(2; 0,2; 2,0; 0,2; 2; 2,0; 0,2), vec![
        c![0, 0], c![2, 2], c![2, 4, 1], 
        c![3, 4], c![4, 4], c![4, 6, 1], 
        c![5, 7], c![6, 8], c![7, 8], 
        c![8, 8], c![8, 10, 1]
    ])
    ?; Ok(())
}

#[test]
fn stays_on_its_own_side_of_replacements() -> Result<(), Report> {
    assert_postions(sec!(2; 2,2; 2), vec![
        c![2, 2, 1],  c![2, 2, -1], c![2, 2, D], c![2, 2, B], c![2,  , A],
        c![3, 2, -1], c![3, 4, 1],  c![3,  , D], c![3,  , B], c![3,  , A],
        c![4, 4, 1],  c![4, 4, -1], c![4, 4, D], c![4,  , B], c![4, 4, A]
    ])
    ?; Ok(())
}

#[test]
fn maps_through_insertions_around_replacements() -> Result<(), Report> {
    assert_postions(sec!(0,1; 2,2; 0,1), vec![
        c![0, 0, -1], c![0, 1, 1],
        c![1, 1, -1], c![1, 3, 1],
        c![2, 3, -1], c![2, 4, 1],
    ])
    ?; Ok(())
}

#[test]
fn stays_in_between_replacements() -> Result<(), Report> {
    assert_postions(sec!(2,2; 2,2), vec![c![2, 2, -1], c![2, 2, 1]])
    ?; Ok(())
}
