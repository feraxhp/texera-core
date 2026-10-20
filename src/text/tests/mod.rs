use mynox::Report;

use crate::Text;


fn line() -> String { "1234567890".repeat(10) }
fn lines() -> Vec<String> { 
    let mut vec = Vec::with_capacity(200);
    vec.resize_with(200, line);
    vec
}

fn text0() -> String { lines().join("\n") }
fn doc0() -> Text  { Text::from(lines().as_slice()) }

#[test]
fn handles_basic_replacement() -> Result<(), Report> {
    let doc = Text::from(["one", "two", "three"]);
    
    let string = doc.replace(Some(2), Some(5), Text::from(["foo", "bar"]));
    assert_eq!(&string.to_string(), "onfoo\nbarwo\nthree");
    Ok(())
}

#[test]
fn can_append_documents() -> Result<(), Report> {
    let left = Text::from(["one", "two", "three"]);
    let right = Text::from(["!", "ok"]);
    
    let string = left.append(right);
    assert_eq!(&string.to_string(), "one\ntwo\nthree!\nok");
    Ok(())
}

#[test]
fn preserves_length() -> Result<(), Report> {
    assert_eq!(text0().len(), doc0().to_string().len());
    Ok(())
}

// creates_a_balanced_tree_when_loading_a_document is managed by rope

#[test]
fn collapses_on_delete() -> Result<(), Report> {
    let doc = doc0().replace(Some(10), Some(text0().len() - 10), Text::default());
    assert_eq!(doc.len_bytes(), 20);
    assert_eq!(doc.to_string(), line()[0..20]);
    Ok(())
}

#[test]
fn handles_deleting_at_start() -> Result<(), Report> {
    let mut lines = lines();
    // Reemplaza la última línea (equivalente a lines.slice(0, -1).concat(...))
    *lines.last_mut().unwrap() = format!("{}!", line());
    
    let left = Text::from(lines.as_slice())
        .replace(None, Some(9500), Text::default())
        .to_string();
    
    // text0 desde 9500 hasta el final, más el "!" que tiene la última línea
    let right = format!("{}!", &text0()[9500..]);
    
    assert_eq!(left, right);
    Ok(())
}

#[test]
fn handles_deleting_at_end() -> Result<(), Report> {
    let mut lines = lines();
    lines[0] = format!("?{}", line());
    
    let string = Text::from(lines.as_slice())
        .replace(Some(9500), Some(text0().len() + 1), Text::default())
        .to_string();
    
    assert_eq!(string, format!("?{}", &text0()[0..9499]));
    Ok(())
}

#[test]
fn can_handle_deleting_the_entire_document() -> Result<(), Report> {
    let string = doc0()
        .replace(Some(0), Some(doc0().len_chars()), Text::default())
        .to_string();
    
    assert_eq!(string, "");
    Ok(())
}

// can_insert_on_node_boundaries managed by ropey

#[test]
fn can_build_up_a_doc_by_repeated_appending() -> Result<(), Report> {
    let mut doc = Text::default();
    let mut text = String::new();
    
    for i in 1..1000 {
        let add = format!("newtext{i} ");
        let len = doc.len_chars();
        doc = doc.replace(Some(len), Some(len), Text::from(add.as_str()));
        text.push_str(&add);
    }
    
    assert_eq!(doc.to_string(), text);
    Ok(())
}

#[test]
fn properly_maintains_content_during_editing() -> Result<(), Report> {
    let mut str = text0();
    let mut doc = doc0();
    let mut seed = 123456789u64;
    let mut next_rand = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (seed >> 33) as usize
    };
    
    for _ in 0..200 {
        let ins_pos = next_rand() % (doc.len_chars() + 1);
        let ins_char = (b'A' + (next_rand() % 26) as u8) as char;
        
        str.insert(ins_pos, ins_char);
        doc = doc.replace(Some(ins_pos), Some(ins_pos), Text::from(ins_char.to_string().as_str()));
        
        let del_from = next_rand() % doc.len_chars();
        let del_to = (del_from + (next_rand() % 20)).min(doc.len_chars());
        
        str.drain(del_from..del_to);
        doc = doc.replace(Some(del_from), Some(del_to), Text::default());
    }
    
    assert_eq!(doc.to_string(), str);
    Ok(())
}

#[test]
fn returns_the_correct_strings_for_slice() -> Result<(), Report> {
    let mut text = Vec::with_capacity(1000);
    for i in 0..1000 {
        text.push(format!("{i:04}"));
    }
    
    let doc = Text::from(text.as_slice());
    let str = text.join("\n");
    
    let mut seed = 987654321u64;
    let mut next_rand = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (seed >> 33) as usize
    };
    
    for i in 0..400 {
        let start = if i == 0 { 0 } else { next_rand() % doc.len_chars() };
        let end = if i == 399 { doc.len_chars() } else { start + (next_rand() % (doc.len_chars() - start + 1)) };
        
        assert_eq!(doc.slice(start, Some(end)).to_string(), &str[start..end]);
    }
    Ok(())
}

#[test]
fn can_be_compared() -> Result<(), Report> {
    let doc = doc0();
    let doc2 = Text::from(lines().as_slice());
    
    assert_eq!(doc, doc.clone());
    assert_eq!(doc, doc2);
    assert_eq!(doc2, doc);
    assert_ne!(doc, doc2.clone().replace(Some(5000), Some(5000), Text::from("y")));
    assert_ne!(doc, doc2.clone().replace(Some(5000), Some(5001), Text::from("y")));
    assert_eq!(doc, doc.clone().replace(Some(5000), Some(5001), doc.slice(5000, Some(5001))));
    assert_ne!(doc, doc.clone().replace(Some(5000), Some(5001), Text::from("y")));
    Ok(())
}

#[test]
fn can_be_compared_despite_different_tree_shape() -> Result<(), Report> {
    let mut lines_expected = Vec::with_capacity(lines().len());
    lines_expected.push(format!("{}abc", line()));
    lines_expected.extend_from_slice(&lines()[2..]);
    
    let left = doc0()
        .replace(Some(100), Some(201), Text::from("abc"));
    let right = Text::from(lines_expected.as_slice());
    
    assert_eq!(left, right);
    Ok(())
}

#[test]
fn can_compare_small_documents() -> Result<(), Report> {
    assert_eq!(Text::from(["foo", "bar"]), Text::from(["foo", "bar"]));
    assert_ne!(Text::from(["foo", "bar"]), Text::from(["foo", "baz"]));
    Ok(())
}

#[test]
fn is_iterable() -> Result<(), Report> {
    let build: String = doc0().chars().collect();
    assert_eq!(build, text0());
    Ok(())
}

#[test]
fn is_partially_iterable() -> Result<(), Report> {
    let doc = doc0();
    let len = doc.len_chars();
    let found = doc.slice(500, Some(len - 500)).to_string();
    let expected = &text0()[500..(text0().len() - 500)];
    
    assert_eq!(found, expected);
    Ok(())
}

#[test]
fn can_iterate_over_lines() -> Result<(), Report> {
    let doc = Text::from(["ab", "cde", "", "", "f", "", "g"]);
    
    let get = |from: Option<usize>, to: Option<usize>| -> String {
        let lines: Vec<String> = doc.iter_lines(from, to).map(|l| l.text()).collect();
        lines.concat()
    };
    
    assert_eq!(get(None, None), "ab\ncde\n\n\nf\n\ng");
    assert_eq!(get(Some(0), Some(doc.len_lines())), "ab\ncde\n\n\nf\n\ng");
    assert_eq!(get(Some(1), Some(1)), "cde\n");
    assert_eq!(get(Some(0), Some(3)), "ab\ncde\n\n\n");
    assert_eq!(get(Some(2), Some(0)), "");
    assert_eq!(get(Some(2), None), "\n\nf\n\ng");
    Ok(())
}

#[test]
fn can_get_line_info_by_line_number() -> Result<(), Report> {
    let doc = doc0();
    assert!(doc.line(doc.len_lines()).is_err());
    assert!(doc.line(doc.len_lines() + 1).is_err());
    
    for i in (0..doc.len_lines()).step_by(5) {
        let l = doc.line(i)?;
        assert_eq!(l.from(), i * 101);
        assert_eq!(l.number(), i);
        assert_eq!(l.text_trimmed().len(), 100);
        assert_eq!(l.length_trimmed(), 100);
        assert_eq!(l.to_trimmed(), i * 101 + 100);
    }
    Ok(())
}

#[test]
fn can_get_line_info_by_position() -> Result<(), Report> {
    let doc = doc0();
    assert!(doc.line_at(doc.len_chars() + 1).is_err());
    
    for i in (0..doc.len_chars()).step_by(5) {
        let l = doc.line_at(i)?;
        assert_eq!(l.from(), i - (i % 101));
        assert_eq!(l.number(), i / 101);
    }
    Ok(())
}

#[test]
fn can_retrieve_pieces_of_text() -> Result<(), Report> {
    let doc = doc0();
    let str = text0();
    let len = doc.len_chars();
    
    let mut seed = 555555u64;
    let mut next_rand = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (seed >> 33) as usize
    };
    
    for _ in 0..500 {
        let from = next_rand() % (len - 1);
        let to = if (next_rand() % 2) == 0 {
            (from + 2).min(len)
        } else {
            (from + (next_rand() % (len - 1 - from)) + 1).min(len)
        };
        
        assert_eq!(doc.slice_string(from, Some(to), None), &str[from..to]);
        assert_eq!(doc.slice(from, Some(to)).to_string(), &str[from..to]);
    }
    Ok(())
}

#[test]
fn clips_out_of_range_boundaries() -> Result<(), Report> {
    let doc = doc0();
    assert_eq!(doc.slice(5, Some(0)).len_chars(), 0);
    assert_eq!(Text::default().slice(0, Some(10)).len_chars(), 0);
    assert_eq!(Text::default().slice(1000, Some(1100)).len_chars(), 0);
    Ok(())
}
