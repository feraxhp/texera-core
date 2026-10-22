use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;


/// Finds the next or previous grapheme cluster boundary in `string` starting from character index `pos`.
///
/// - `pos` is a **character offset** (`char` count, not UTF-8 byte offset).
/// - If `forward` is `true`, moves to the end of the cluster starting at or containing `pos`.
///   If `forward` is `false`, moves to the start of the cluster ending at or containing `pos`.
/// - When `include_extending` is `true`, full Unicode extended grapheme clusters (such as
///   combining marks or ZWJ emoji sequences) are kept together as a single unit. When `false`,
///   steps by a single `char` code point.
///
/// # Examples
///
/// ```
/// use texera_core::find_cluster_break;
///
/// // 'e' followed by combining acute accent '\u{301}' (2 chars, 1 grapheme cluster)
/// let text = "e\u{301}x";
/// assert_eq!(text.chars().count(), 3);
///
/// // Moving forward across the whole grapheme cluster:
/// assert_eq!(find_cluster_break(text, 0, true, true), 2);
/// // Moving forward by a single char (ignoring extending marks):
/// assert_eq!(find_cluster_break(text, 0, true, false), 1);
/// // Moving backward from index 2 across the cluster:
/// assert_eq!(find_cluster_break(text, 2, false, true), 0);
/// ```
pub fn find_cluster_break(
    string: &str,
    pos: usize,
    forward: bool,
    include_extending: bool,
) -> usize {
    if string.is_ascii() {
        let len = string.len();
        return if forward {
            if pos >= len { pos } else { pos + 1 }
        } else {
            if pos == 0 { 0 } else { pos.min(len) - 1 }
        };
    }

    if forward {
        if !include_extending {
            let total_chars = string.chars().count();
            return if pos >= total_chars { pos } else { pos + 1 };
        }
        let mut char_offset = 0;
        for g in string.graphemes(true) {
            let next_offset = char_offset + g.chars().count();
            if pos < next_offset {
                return next_offset;
            }
            char_offset = next_offset;
        }
        char_offset.max(pos)
    } else {
        if pos == 0 { return 0; }
        if !include_extending {
            let total_chars = string.chars().count();
            return pos.min(total_chars) - 1;
        }
        let mut char_offset = 0;
        for g in string.graphemes(true) {
            let next_offset = char_offset + g.chars().count();
            if pos <= next_offset {
                return char_offset;
            }
            char_offset = next_offset;
        }
        char_offset
    }
}

/// Counts the visual column width of `string`, expanding `'\t'` characters to the next
/// multiple of `tab_size` and accounting for full-width Unicode characters.
///
/// # Examples
///
/// ```
/// use texera_core::count_column;
///
/// assert_eq!(count_column("hello", 4), 5);
/// assert_eq!(count_column("\thello", 4), 9); // '\t' expands to 4 columns
/// assert_eq!(count_column("ab\tc", 4), 5);   // "ab" (2) + '\t' (2) + "c" (1)
/// ```
pub fn count_column(string: &str, tab_size: usize) -> usize {
    let tab_size = tab_size.max(1);
    if string.is_ascii() && !string.bytes().any(|b| b == b'\t' || b == b'\n' || b == b'\r') {
        return string.len();
    }
    let mut col = 0;
    for g in string.graphemes(true) {
        if g == "\t" {
            col += tab_size - (col % tab_size);
        } else if g == "\n" || g == "\r" || g == "\r\n" {
            continue;
        } else {
            col += UnicodeWidthStr::width(g).max(1);
        }
    }
    col
}

/// Finds the character offset in `string` that corresponds to visual column `col`
/// (given `tab_size`), returning `(char_offset, actual_column)`.
///
/// If `col` exceeds the visual width of `string`, returns the total character length
/// and total visual column width of `string`.
///
/// # Examples
///
/// ```
/// use texera_core::find_column;
///
/// // "\thello" with tab_size = 4: '\t' spans columns 0..4, 'h' is at column 4 (char index 1)
/// assert_eq!(find_column("\thello", 4, 4), (1, 4));
/// assert_eq!(find_column("\thello", 6, 4), (3, 6));
/// ```
pub fn find_column(string: &str, col: usize, tab_size: usize) -> (usize, usize) {
    let tab_size = tab_size.max(1);
    let mut char_idx = 0;
    let mut cur_col = 0;
    for g in string.graphemes(true) {
        if cur_col >= col {
            return (char_idx, cur_col);
        }
        if g == "\t" {
            cur_col += tab_size - (cur_col % tab_size);
        } else if g != "\n" && g != "\r" && g != "\r\n" {
            cur_col += UnicodeWidthStr::width(g).max(1);
        }
        char_idx += g.chars().count();
    }
    (char_idx, cur_col)
}
