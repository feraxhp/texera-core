use texera_core::ChangeSpec;


#[test]
fn valid() {
    let change = ChangeSpec::new(20, Some(21), None);
    
    assert!(change.is_ok())
}

#[test]
fn valid_eq() {
    let change = ChangeSpec::new(20, None, None);
    
    assert!(change.is_ok())
}

#[test]
fn invalid() {
    let change = ChangeSpec::new(20, Some(19), None);
    
    assert!(change.is_err())
}
