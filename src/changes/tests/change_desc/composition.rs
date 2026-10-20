use mynox::Error;
use mynox::Report;

use crate::sec;
use crate::ErrorTypes;

use crate::changes::ChangeDesc;
use crate::changes::Composable;
use crate::changes::Section;

/// - a is the first set of changes
/// - b is the second set of changes
/// - c is the expected composed set of changes
fn compare_changes(a: Vec<Section>, b: Vec<Section>, c: Vec<Section>) -> Result<Vec<Section>, Error> {
    let init = ChangeDesc { sections: a };
    let second = ChangeDesc { sections: b };
    
    let result = init.compose(&second)?;
    
    assert_eq!(result.sections, c);
    Ok(result.sections)
}

fn expect_error(a: Vec<Section>, b: Vec<Section>) -> Result<(), Report> {
    let init = ChangeDesc { sections: a };
    let second = ChangeDesc { sections: b };
    
    let result = init.compose(&second);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().etype(), ErrorTypes::DifferingChanges.get_etype());
    Ok(())
}

#[test]
fn can_compose_unrelated_changes() -> Result<(), Report> {
    compare_changes(sec!( 5; 0,2; ), sec!(  1; 2,0; 4; ), sec!(  1; 2,0; 2; 0,2; ) )
    ?; Ok(())
}

#[test]
fn cancels_insertions_with_deletions() -> Result<(), Report> {
    compare_changes(sec!( 2; 0,2; 2; ), sec!(  2; 2,0; 2; ), sec!( 4; ) )
    ?; Ok(())
}

#[test]
fn joins_adjacent_insertions()  -> Result<(), Report> {
    compare_changes(sec!(2; 0,2; 2; ), sec!( 4; 0,3; 2; ) , sec!( 2; 0,5; 2; ))
    ?; Ok(())
}

#[test]
fn joins_adjacent_deletions()  -> Result<(), Report> {
    compare_changes(sec!(2; 5,0), sec!(1; 1,0), sec!(1; 6,0))
    ?; Ok(())
}

#[test]
fn allows_a_delete_to_shadow_multiple_operations()  -> Result<(), Report> {
    compare_changes(sec!(2; 2,0; 0,3), sec!(5,0), sec!(4,0))
    ?; Ok(())
}

#[test]
fn can_handle_empty_sets()  -> Result<(), Report> {
        let result = compare_changes(sec!(), sec!(0,8), sec!(0,8))
    ?; let result = compare_changes(result, sec!(8, 0), sec!())
    ?; compare_changes(result, sec!(), sec!())
    ?; Ok(())
}

#[test]
fn can_join_multiple_replaces()  -> Result<(), Report> {
        compare_changes(sec!(2; 2,2; 2,2; 2),      sec!(1; 2,2; 2,2; 2,2; 1), sec!(1; 6,6; 1))
    ?; compare_changes(sec!(1; 2,2; 2,2; 2,2; 1), sec!(2; 2,2; 2,2; 2),      sec!(1; 6,6; 1))
    ?; compare_changes(sec!(1; 2,3; 3,2; 1),      sec!(2; 3,1; 2),           sec!(1; 5,3; 1))
    ?; Ok(())
}

#[test]
fn throws_for_inconsistent_lengths() -> Result<(), Report> {
        expect_error(sec!( 2; 0,2 ), sec!( 1; 0,1 ))
    ?; expect_error(sec!( 2; 0,2 ), sec!( 30; 0,1 ))
    ?; expect_error(sec!( 2; 2,0; 0,3 ), sec!( 7,0 ))
    ?; expect_error(sec!( 5 ), sec!( 4 ))
    ?; expect_error(sec!(), sec!( 5 ))
    ?; expect_error(sec!( 5 ), sec!())
    ?; Ok(())
}
