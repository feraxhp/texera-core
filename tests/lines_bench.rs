use std::time::Instant;
use texera_core::Text;

#[test]
fn bench_line_iteration() {
    let num_lines = 100_000;
    let line = "fn some_function_name_here() -> Result<(), Box<dyn Error>> {\n";
    let doc_str = line.repeat(num_lines);
    let doc = Text::from(doc_str.as_str());

    // 1. Direct RopeSlice iteration via deref
    let start = Instant::now();
    let mut total_len = 0;
    for slice in doc.lines() {
        total_len += slice.len_chars();
    }
    let dur_ropey = start.elapsed();
    println!("\n>>> ropey doc.lines() 100k lines: {:?} (len: {})", dur_ropey, total_len);

    // 2. Text::iter_lines()
    let start = Instant::now();
    let mut total_len2 = 0;
    for l in doc.iter_lines(None, None) {
        total_len2 += l.length();
    }
    let dur_texera = start.elapsed();
    println!(">>> doc.iter_lines() 100k lines: {:?} (len: {})", dur_texera, total_len2);

    assert_eq!(total_len, total_len2);

    // 3. Text::iter_lines() with text_trimmed() check
    let start = Instant::now();
    let mut total_trimmed_len = 0;
    for l in doc.iter_lines(None, None) {
        total_trimmed_len += l.length_trimmed();
    }
    let dur_trimmed = start.elapsed();
    println!(">>> doc.iter_lines() length_trimmed 100k lines: {:?} (len: {})", dur_trimmed, total_trimmed_len);
}
