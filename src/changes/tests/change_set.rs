use std::sync::OnceLock;

use mynox::Report;

use crate::changes::Change;
use crate::changes::ChangeIterator;
use crate::changes::ChangeMap;
use crate::changes::Composable;
use crate::changes::ChangeSet;
use crate::changes::ChangeSpec;
use crate::changes::Range;
use crate::changes::Section;
use crate::changes::Mapable;
use crate::Text;
use crate::sec;
use crate::ch;


fn doc10() -> Text {
    static DOC: OnceLock<Text> = OnceLock::new();
    
    DOC.get_or_init(|| {
        Text::from("0123456789").clone()
    }).clone()
}


macro_rules! change {
    ($a:literal, $b:literal, $c:literal, $d:literal, $e:literal ) => {
        Change { map: ChangeMap { old: Range { from: $a, to: $b }, new: Range { from: $c, to: $d } }, text: Text::from($e) }
    };
}

#[test]
fn can_create_change_sets() -> Result<(), Report> {
    assert_eq!( ChangeSet::from_slice(&ch!([{ from: 5, insert: "hi" }]), 10)?.sections(), &sec!(5; 0,2; 5));
    assert_eq!( ChangeSet::from_slice(&ch!([{ from: 5, to: 7 }]), 10)?.sections(), &sec!(5; 2,0; 3));
    assert_eq!( ChangeSet::from_slice(&ch!([
      {from: 5, insert: "hi"}, {from: 5, insert: "ok"},
      {from: 0, to: 3}, {from: 4, to: 6},
      {insert: "boo", from: 8}
    ]), 10)?.sections(), &sec!(3,0; 1; 1,0; 0,4; 1,0; 2; 0,3; 2));
    Ok(())
}

#[test]
fn can_apply_change_sets() -> Result<(), Report> {
    assert_eq!(ChangeSet::from_slice(&ch!([{insert: "ok", from: 2}]), 10)?.apply(doc10())?.to_string(), "01ok23456789");
    assert_eq!(ChangeSet::from_slice(&ch!([{from: 1, to: 9}]), 10)?.apply(doc10())?.to_string(), "09");
    assert_eq!(ChangeSet::from_slice(&ch!([{from: 2, to: 8}, {insert: "hi", from: 1}]), 10)?.apply(doc10())?.to_string(), "0hi189");
    Ok(())
}

#[test]
fn can_apply_composed_sets() -> Result<(), Report> {
    assert_eq!(
        ChangeSet::from_slice(&ch!([{insert: "ABCD", from: 8}]), 10)?
            .compose(
                &ChangeSet::from_slice(&ch!([{from: 8, to: 11}]), 14)?
            )?
            .apply(doc10())?.to_string(), 
        "01234567D89"
    );
    
    assert_eq!(
        ChangeSet::from_slice(&ch!([{insert: "hi", from: 2}, {insert: "ok", from: 8}]), 10)?
            .compose(
                &ChangeSet::from_slice(&ch!([{insert: "!", from: 4}, {from: 6, to: 8}, {insert: "?", from: 12}]), 14)?
            )?
            .apply(doc10())?.to_string(), 
        "01hi!2367ok?89"
    );
    
    Ok(())
}

#[test]
fn can_clip_inserted_strings_on_compose() -> Result<(), Report> {
    assert_eq!(
        ChangeSet::from_slice(&ch!([{insert: "abc", from: 2}, {insert: "def", from: 4}]), 10)?
            .compose(
                &ChangeSet::from_slice(&ch!([{from: 4, to: 8}]), 16)?
            )?
            .apply(doc10())?.to_string(), 
        "01abef456789"
    );
    Ok(())
}

#[test]
fn can_apply_mapped_sets() -> Result<(), Report> {
    let set0 = ChangeSet::from_slice(&ch!([{insert: "hi", from: 5}, {from: 8, to: 10}]), 10)?;
    let set1 = ChangeSet::from_slice(&ch!([{insert: "ok", from: 10}, {from: 6, to: 7}]), 10)?;
    assert_eq!(set0.compose(&set1.map(&set0, false)?)?.apply(doc10())?.to_string(), "01234hi57ok");
    Ok(())
}

#[test]
fn can_apply_inverted_sets() -> Result<(), Report> {
    let set0 = ChangeSet::from_slice(&ch!([{insert: "hi", from: 5}, {from: 8, to: 10}]), 10)?;
    assert_eq!(set0.invert(&doc10()).apply(set0.apply(doc10())?)?.to_string(), doc10().to_string());
    Ok(())
}

#[test]
fn can_be_iterated() -> Result<(), Report> {
    let set = ChangeSet::from_slice(&ch!([{insert: "ok", from: 4}, {from: 6, to: 8}]), 10)?;
    let result: Vec<_> = set.iter_changes(false).collect();
    assert_eq!(result, vec![change![4, 4, 4, 6, "ok"], change![6, 8, 8, 8, ""]]);
    
    let result: Vec<_> = set.iter_unchanged().map(|s| [ s.map.old, s.map.new, s.length ]).collect();
    assert_eq!(result, [[0, 0, 4], [4, 6, 2], [8, 8, 2]]);
    Ok(())
}


// ('r' for random)
struct Rand {
    seed: u64,
}

impl Rand {
    fn new(seed: u64) -> Self {
        Self { seed }
    }
    
    fn r(&mut self, n: usize) -> usize {
        if n == 0 { return 0; }
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((self.seed >> 33) as usize) % n
    }
    
    fn r_str(&mut self, l: usize) -> String {
        let mut result = String::with_capacity(l);
        for _ in 0..l {
            result.push((b'a' + (self.r(26) as u8)) as char);
        }
        result
    }
    
    fn r_change(&mut self, len: usize) -> ChangeSpec {
        if len == 0 || self.r(3) == 0 {
            let from = if len == 0 { 0 } else { self.r(len) };
            let str_len = self.r(5) + 1;
            let str = self.r_str(str_len);
            ChangeSpec::new(from, None, Some(Text::from(str.as_str()))).unwrap()
        } else {
            let from = if len > 1 { self.r(len - 1) } else { 0 };
            let to = (from + self.r(5) + 1).min(len);
            let insert = if self.r(2) == 0 {
                let str_len = self.r(2) + 1;
                let str = self.r_str(str_len);
                Some(Text::from(str.as_str()))
            } else {
                None
            };
            ChangeSpec::new(from, Some(to), insert).unwrap()
        }
    }
    
    fn r_changes(&mut self, len: usize, count: usize) -> Vec<ChangeSpec> {
        let mut result = Vec::with_capacity(count);
        for _ in 0..count {
            result.push(self.r_change(len));
        }
        result
    }
}

#[test]
fn mapping_before_produces_the_same_result_as_mapping_the_other_after() -> Result<(), Report> {
    let mut rng = Rand::new(12345);
    let total = 100;
    for i in 0..total {
        let size = rng.r(20);
        let count = i / (total / 10) + 1;
        let a = rng.r_changes(size, count);
        let b = rng.r_changes(size, count);
        let set_a = ChangeSet::from_slice(&a, size)?;
        let set_b = ChangeSet::from_slice(&b, size)?;
        let set_a1 = set_a.map(&set_b, true)?;
        let set_b1 = set_b.map(&set_a, false)?;
        let doc = Text::from(rng.r_str(size).as_str());
        let set_ab = set_a.compose(&set_b1)?;
        let set_ba = set_b.compose(&set_a1)?;
        assert_eq!(set_ab.apply(doc.clone())?.to_string(), set_ba.apply(doc)?.to_string());
    }
    Ok(())
}

#[test]
fn mapping_still_converges_when_mapping_through_multiple_changes() -> Result<(), Report> {
    let mut rng = Rand::new(23456);
    let total = 100;
    for i in 0..total {
        let size = rng.r(20);
        let count = i / (total / 10) + 1;
        let changes_a = rng.r_changes(size, count);
        let a = ChangeSet::from_slice(&changes_a, size)?;
        let changes_b = rng.r_changes(a.len_new(), count);
        let b = ChangeSet::from_slice(&changes_b, a.len_new())?;
        let changes_c = rng.r_changes(size, count);
        let c = ChangeSet::from_slice(&changes_c, size)?;
        let c_a = c.map(&a, false)?;
        let c_ab = c_a.map(&b, false)?;
        let a_c = a.map(&c, true)?;
        let b_ca = b.map(&c_a, true)?;
        let doc = Text::from(rng.r_str(size).as_str());
        assert_eq!(
            a.compose(&b)?.compose(&c_ab)?.apply(doc.clone())?.to_string(),
            c.compose(&a_c)?.compose(&b_ca)?.apply(doc)?.to_string()
        );
    }
    Ok(())
}

#[test]
fn compose_produces_the_same_result_as_individual_changes() -> Result<(), Report> {
    let mut rng = Rand::new(34567);
    for _ in 0..100 {
        let size = rng.r(20);
        let doc = Text::from(rng.r_str(size).as_str());
        let count_a = rng.r(5) + 1;
        let changes_a = rng.r_changes(size, count_a);
        let a = ChangeSet::from_slice(&changes_a, size)?;
        let count_b = rng.r(6);
        let changes_b = rng.r_changes(a.len_new(), count_b);
        let b = ChangeSet::from_slice(&changes_b, a.len_new())?;
        assert_eq!(
            b.apply(a.apply(doc.clone())?)?.to_string(),
            a.compose(&b)?.apply(doc)?.to_string()
        );
    }
    Ok(())
}

#[test]
fn composing_is_associative() -> Result<(), Report> {
    let mut rng = Rand::new(45678);
    for _ in 0..100 {
        let size = rng.r(20);
        let doc = Text::from(rng.r_str(size).as_str());
        let count_a = rng.r(5) + 1;
        let changes_a = rng.r_changes(size, count_a);
        let a = ChangeSet::from_slice(&changes_a, size)?;
        let count_b = rng.r(6);
        let changes_b = rng.r_changes(a.len_new(), count_b);
        let b = ChangeSet::from_slice(&changes_b, a.len_new())?;
        let count_c = rng.r(5) + 1;
        let changes_c = rng.r_changes(b.len_new(), count_c);
        let c = ChangeSet::from_slice(&changes_c, b.len_new())?;
        let left = a.compose(&b)?.compose(&c)?;
        let right = a.compose(&b.compose(&c)?)?;
        assert_eq!(left.apply(doc.clone())?.to_string(), right.apply(doc)?.to_string());
    }
    Ok(())
}

#[test]
fn survives_random_sequences_of_changes() -> Result<(), Report> {
    let mut rng = Rand::new(56789);
    for _ in 0..50 {
        let mut doc = doc10();
        let mut txt = doc.to_string();
        let mut all: Vec<ChangeSet> = Vec::new();
        let mut inv: Vec<ChangeSet> = Vec::new();
        
        for _ in 0..50 {
            let change = rng.r_change(doc.len_chars());
            let from = change.from;
            let to = change.to;
            let insert_str = change.insert.as_ref().map(|t| t.to_string()).unwrap_or_default();
            txt = format!("{}{}{}", &txt[..from], insert_str, &txt[to..]);
            let set = ChangeSet::from_slice(&[change], doc.len_chars())?;
            all.push(set.clone());
            inv.push(set.invert(&doc));
            doc = set.apply(doc)?;
            assert_eq!(doc.to_string(), txt);
        }
        
        let mut composed = ChangeSet::from_slice(&[], doc10().len_chars())?;
        for s in &all {
            composed = composed.compose(s)?;
        }
        assert_eq!(composed.apply(doc10())?.to_string(), txt);
        assert_eq!(composed.invert(&doc10()).apply(doc.clone())?.to_string(), doc10().to_string());
        for s in inv.iter().rev() {
            doc = s.apply(doc)?;
        }
        assert_eq!(doc.to_string(), doc10().to_string());
    }
    Ok(())
}

// can_be_serialized_to_json is not applicable (requires serde).
