use mynox::Error;
use mynox::Report;

use crate::sec;
use crate::ErrorTypes;
use crate::changes::ChangeDesc;
use crate::changes::Section;
use crate::changes::Mapable;


/// - a is the first set of changes
/// - b is the second set of changes
/// - c is the expected composed set of changes
fn over(a: Vec<Section>, b: Vec<Section>, c: Vec<Section>) -> Result<Vec<Section>, Error> {
    let init = ChangeDesc { sections: a };
    let second = ChangeDesc { sections: b };
    
    let result = init.map(&second, false)?;
    
    assert_eq!(result.sections, c);
    Ok(result.sections)
}

/// - a is the first set of changes
/// - b is the second set of changes
/// - c is the expected composed set of changes
fn under(a: Vec<Section>, b: Vec<Section>, c: Vec<Section>) -> Result<Vec<Section>, Error> {
    let init = ChangeDesc { sections: a };
    let second = ChangeDesc { sections: b };
    
    let result = init.map(&second, true)?;
    
    assert_eq!(result.sections, c);
    Ok(result.sections)
}

fn over_err(a: Vec<Section>, b: Vec<Section>) -> Result<(), Report> {
    let init = ChangeDesc { sections: a };
    let second = ChangeDesc { sections: b };
    
    let result = init.map(&second, false);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().etype(), ErrorTypes::DifferingChanges.get_etype());
    Ok(())
}

fn under_err(a: Vec<Section>, b: Vec<Section>) -> Result<(), Report> {
    let init = ChangeDesc { sections: a };
    let second = ChangeDesc { sections: b };
    
    let result = init.map(&second, true);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().etype(), ErrorTypes::DifferingChanges.get_etype());
    Ok(())
}

#[test]
fn can_map_over_an_insertion() -> Result<(), Report> {
        over(sec!( 4; 0,1 ), sec!( 0,3; 4 ) , sec!( 7; 0,1 ))
    ?; Ok(())
}

#[test]
fn can_map_over_a_deletion() -> Result<(), Report> {
        over(sec!( 4; 0,1 ), sec!( 2,0; 2 ), sec!( 2; 0,1 ))
    ?; Ok(())
}

#[test]
fn orders_insertions() -> Result<(), Report> {
        over(sec!( 2; 0,1; 2 ), sec!( 2; 0,1; 2 ), sec!( 3; 0,1; 2 ))
    ?; under(sec!( 2; 0,1; 2 ), sec!( 2; 0,1; 2 ), sec!( 2; 0,1; 3 ))
    ?; Ok(())
}

#[test]
fn can_map_a_deletion_over_an_overlapping_replace() -> Result<(), Report> {
        over(sec!( 2; 2,0 ), sec!( 2; 1,2; 1 ), sec!( 4; 1,0 ))
    ?; under(sec!( 2; 2,0 ), sec!( 2; 1,2; 1 ), sec!( 4; 1,0 ))
    ?; Ok(())
}

#[test]
fn can_handle_changes_after() -> Result<(), Report> {
        over(sec!( 0,1; 2,0; 8 ), sec!( 6; 1,0; 0,5; 3 ), sec!( 0,1; 2,0; 12 ))
    ?; Ok(())
}

#[test]
fn joins_deletions() -> Result<(), Report> {
        over(sec!( 5,0; 2; 3,0; 2 ), sec!( 4; 4,0; 4 ), sec!( 6,0; 2 ))
    ?; Ok(())
}

#[test]
fn keeps_insertions_in_deletions() -> Result<(), Report> {
        under(sec!( 2; 0,1; 2 ), sec!( 4,0 ), sec!( 0,1 ))
    ?; over(sec!( 4; 0,1; 4 ), sec!( 2; 4,0; 2 ), sec!( 2; 0,1; 2 ))
    ?; Ok(())
}

#[test]
fn keeps_replacements() -> Result<(), Report> {
        over(sec!( 2; 2,2; 2 ), sec!( 0,2; 6 ), sec!( 4; 2,2; 2 ))
    ?; over(sec!( 2; 2,2; 2 ), sec!( 3,0; 3 ), sec!( 1,2; 2 ))
    ?; over(sec!( 1; 4,4; 1 ), sec!( 3; 0,2; 3 ), sec!( 1; 2,4; 2; 2,0; 1 ))
    ?; over(sec!( 1; 4,4; 1 ), sec!( 2; 2,0; 2 ), sec!( 1; 2,4; 1 ))
    ?; over(sec!( 2; 2,2; 2 ), sec!( 3; 2,0; 1 ), sec!( 2; 1,2; 1 ))
    ?; Ok(())
}

#[test]
fn doesnt_join_replacements() -> Result<(), Report> {
        over(sec!( 2,2; 2; 2,2 ), sec!( 2; 2,0; 2 ), sec!( 2,2; 2,2 ))
    ?; Ok(())
}

#[test]
fn drops_duplicate_deletion() -> Result<(), Report> {
        under(sec!( 2; 2,0; 2 ), sec!( 2; 2,0; 2 ), sec!( 4 ))
    ?; over(sec!( 2; 2,0; 2 ), sec!( 2; 2,0; 2 ), sec!( 4 ))
    ?; Ok(())
}

#[test]
fn handles_overlapping_replaces() -> Result<(), Report> {
        over(sec!( 1; 1,2; 1 ), sec!( 1; 1,1; 1 ), sec!( 2; 0,2; 1 ))
    ?; under(sec!( 1; 1,2; 1 ), sec!( 1; 1,1; 1 ), sec!( 1; 0,2; 2 ))
    ?; over(sec!( 1; 1,2; 2 ), sec!( 1; 2,1; 1 ), sec!( 1; 0,2; 2 ))
    ?; over(sec!( 2; 1,2; 1 ), sec!( 1; 2,1; 1 ), sec!( 2; 0,2; 1 ))
    ?; over(sec!( 2,1; 1 ), sec!( 1; 2,2 ), sec!( 1,1; 2 ))
    ?; over(sec!( 1; 2,1 ), sec!( 2,2; 1 ), sec!( 2; 1,1 ))
    ?; Ok(())
}

#[test]
fn throws_for_inconsistent_lengths() -> Result<(), Report> {
        over_err(sec!( 2; 0,1 ), sec!( 4; 0,1 ))
    ?; under_err(sec!( 2; 0,1 ), sec!( 4; 0,1 ))
    ?; over_err(sec!( 2; 0,2 ), sec!( 1; 0,1 ))
    ?; under_err(sec!( 2; 0,2 ), sec!( 1; 0,1 ))
    ?; over_err(sec!( 2; 0,2 ), sec!( 30; 0,1 ))
    ?; under_err(sec!( 2; 0,2 ), sec!( 30; 0,1 ))
    ?; over_err(sec!( 2; 2,0; 0,3 ), sec!( 7,0 ))
    ?; under_err(sec!( 2; 2,0; 0,3 ), sec!( 7,0 ))
    ?; over_err(sec!( 5 ), sec!( 4 ))
    ?; under_err(sec!( 5 ), sec!( 4 ))
    ?; over_err(sec!(), sec!( 5 ))
    ?; under_err(sec!(), sec!( 5 ))
    ?; Ok(())
}
