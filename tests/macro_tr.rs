use mynox::Report;
use texera_core::{ErrorTypes, Selection, State, tr};


#[test]
fn simple_macro() -> Result<(), Report> {
    let spec = tr!({
        changes: { from: 0, to: 0, insert: "sa" }
    });
    let res = State::from("hello").update(spec)?;
    assert_eq!(res.state().doc().to_string(), "sahello");
    Ok(())
}

#[test]
fn various_changes_macro() -> Result<(), Report> {
    let spec = tr!({
        changes: [
            { from: 0, to: 0, insert: "sa" },
            { from: 2, to: 5, insert: "!" },
        ]
    });
    let res = State::from("hello").update(spec)?;
    assert_eq!(res.state().doc().to_string(), "sahe!");
    Ok(())
}

#[test]
fn simple_macro_err() {
    let build = || -> Result<_, mynox::Error> {
        Ok(tr!({
            changes: { from: 13, to: 12, insert: "sa" }
        }))
    };
    let res = build();
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().etype(), ErrorTypes::RangeError.get_etype());
}

#[test]
fn no_to() -> Result<(), Report> {
    let spec = tr!({
        changes: { from: 2, insert: "dis" }
    });
    let res = State::from("hello").update(spec)?;
    assert_eq!(res.state().doc().to_string(), "hedisllo");
    Ok(())
}

#[test]
fn various_no_to() -> Result<(), Report> {
    let spec = tr!({
        changes: [
            { from: 0, to: 2, insert: "sa" },
            { from: 2, insert: "dis" }
        ]
    });
    let res = State::from("hello").update(spec)?;
    assert_eq!(res.state().doc().to_string(), "sadisllo");
    Ok(())
}

#[test]
fn no_insert() -> Result<(), Report> {
    let spec = tr!({
        changes: { from: 1, to: 4 }
    });
    let res = State::from("hello").update(spec)?;
    assert_eq!(res.state().doc().to_string(), "ho");
    Ok(())
}

#[test]
fn various_no_insert() -> Result<(), Report> {
    let spec = tr!({
        changes: [
            { from: 0, to: 2 },
            { from: 3, to: 5 },
        ]
    });
    let res = State::from("hello").update(spec)?;
    assert_eq!(res.state().doc().to_string(), "l");
    Ok(())
}

#[test]
fn no_to_no_insert() -> Result<(), Report> {
    let spec = tr!({
        changes: { from: 2 }
    });
    let res = State::from("hello").update(spec)?;
    assert_eq!(res.state().doc().to_string(), "hello");
    Ok(())
}

#[test]
fn various_no_to_no_insert() -> Result<(), Report> {
    let spec = tr!({
        changes: [
            { from: 1 },
            { from: 3 },
        ],
        selection: Selection::cursor(2),
        user_event: "input.type",
        scroll_into_view: true,
    });
    let res = State::from("hello").update(spec)?;
    assert_eq!(res.state().doc().to_string(), "hello");
    assert_eq!(res.state().selection().main().head(), 2);
    assert!(res.is_user_event("input"));
    assert!(res.is_user_event("input.type"));
    assert!(!res.is_user_event("input.paste"));
    assert!(res.scroll_into_view());
    Ok(())
}
