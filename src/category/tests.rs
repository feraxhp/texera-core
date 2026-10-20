#[cfg(test)]
mod tests {
    use crate::category::find_cluster_break;
    use crate::category::count_column;
    use crate::category::find_column;
    use crate::CharCategory;


    #[test]
    fn categorises_into_alphanumeric() {
        for c in '0'..='9' {
            assert_eq!(CharCategory::from(c), CharCategory::Word);
        }
        for c in 'a'..='z' {
            assert_eq!(CharCategory::from(c), CharCategory::Word);
        }
        for c in 'A'..='Z' {
            assert_eq!(CharCategory::from(c), CharCategory::Word);
        }
        assert_eq!(CharCategory::from('_'), CharCategory::Word);

        assert_eq!(CharCategory::from('á'), CharCategory::Word);
        assert_eq!(CharCategory::from('é'), CharCategory::Word);
        assert_eq!(CharCategory::from('ñ'), CharCategory::Word);
        assert_eq!(CharCategory::from('ü'), CharCategory::Word);
        assert_eq!(CharCategory::from('Ç'), CharCategory::Word);
        assert_eq!(CharCategory::from('ß'), CharCategory::Word);

        assert_eq!(CharCategory::from('ж'), CharCategory::Word);
        assert_eq!(CharCategory::from('Ω'), CharCategory::Word);
        assert_eq!(CharCategory::from('日'), CharCategory::Word);
        assert_eq!(CharCategory::from('本'), CharCategory::Word);
        assert_eq!(CharCategory::from('語'), CharCategory::Word);
        assert_eq!(CharCategory::from('س'), CharCategory::Word);
        assert_eq!(CharCategory::from('א'), CharCategory::Word);

        assert_eq!(CharCategory::from('٣'), CharCategory::Word);
        assert_eq!(CharCategory::from('४'), CharCategory::Word);
        assert_eq!(CharCategory::from('Ⅶ'), CharCategory::Word);
        assert_eq!(CharCategory::from('²'), CharCategory::Word);
        assert_eq!(CharCategory::from('¼'), CharCategory::Word);
    }

    #[test]
    fn categorises_into_whitespace() {
        assert_eq!(CharCategory::from(' '), CharCategory::Space);
        assert_eq!(CharCategory::from('\t'), CharCategory::Space);
        assert_eq!(CharCategory::from('\n'), CharCategory::Space);
        assert_eq!(CharCategory::from('\r'), CharCategory::Space);
        assert_eq!(CharCategory::from('\x0C'), CharCategory::Space);

        assert_eq!(CharCategory::from('\u{00A0}'), CharCategory::Space);
        assert_eq!(CharCategory::from('\u{2002}'), CharCategory::Space);
        assert_eq!(CharCategory::from('\u{2003}'), CharCategory::Space);
        assert_eq!(CharCategory::from('\u{2009}'), CharCategory::Space);
        assert_eq!(CharCategory::from('\u{202F}'), CharCategory::Space);
        assert_eq!(CharCategory::from('\u{3000}'), CharCategory::Space);
        assert_eq!(CharCategory::from('\u{2028}'), CharCategory::Space);
        assert_eq!(CharCategory::from('\u{2029}'), CharCategory::Space);
    }

    #[test]
    fn categorises_into_other() {
        let punctuation = [
            '/', '<', '>', '.', ',', ';', ':', '!', '?', '@', '#', '$', '%', '^',
            '&', '*', '(', ')', '-', '+', '=', '{', '}', '[', ']', '|', '\\',
            '~', '`', '\'', '"',
        ];
        for &c in &punctuation {
            assert_eq!(CharCategory::from(c), CharCategory::Other);
        }

        assert_eq!(CharCategory::from('—'), CharCategory::Other);
        assert_eq!(CharCategory::from('–'), CharCategory::Other);
        assert_eq!(CharCategory::from('“'), CharCategory::Other);
        assert_eq!(CharCategory::from('”'), CharCategory::Other);
        assert_eq!(CharCategory::from('«'), CharCategory::Other);
        assert_eq!(CharCategory::from('»'), CharCategory::Other);
        assert_eq!(CharCategory::from('¿'), CharCategory::Other);
        assert_eq!(CharCategory::from('¡'), CharCategory::Other);

        assert_eq!(CharCategory::from('€'), CharCategory::Other);
        assert_eq!(CharCategory::from('£'), CharCategory::Other);
        assert_eq!(CharCategory::from('¥'), CharCategory::Other);
        assert_eq!(CharCategory::from('₹'), CharCategory::Other);
        assert_eq!(CharCategory::from('±'), CharCategory::Other);
        assert_eq!(CharCategory::from('×'), CharCategory::Other);
        assert_eq!(CharCategory::from('÷'), CharCategory::Other);
        assert_eq!(CharCategory::from('≠'), CharCategory::Other);
        assert_eq!(CharCategory::from('√'), CharCategory::Other);

        assert_eq!(CharCategory::from('🦀'), CharCategory::Other);
        assert_eq!(CharCategory::from('🚀'), CharCategory::Other);
        assert_eq!(CharCategory::from('⭐'), CharCategory::Other);

        assert_eq!(CharCategory::from('\0'), CharCategory::Other);
        assert_eq!(CharCategory::from('\x07'), CharCategory::Other);
        assert_eq!(CharCategory::from('\x08'), CharCategory::Other);
        assert_eq!(CharCategory::from('\x1B'), CharCategory::Other);
    }

    #[test]
    fn cluster_breaks_and_columns() {
        let s = "e\u{0301}x";
        assert_eq!(find_cluster_break(s, 0, true, true), 2);
        assert_eq!(find_cluster_break(s, 0, true, false), 1);
        assert_eq!(find_cluster_break(s, 2, false, true), 0);
        assert_eq!(find_cluster_break(s, 2, false, false), 1);

        let family = "👨\u{200D}👩\u{200D}👧\u{200D}👦";
        assert_eq!(family.chars().count(), 7);
        assert_eq!(find_cluster_break(family, 0, true, true), 7);
        assert_eq!(find_cluster_break(family, 7, false, true), 0);

        assert_eq!(count_column("a\tbc", 4), 6);
        assert_eq!(find_column("a\tbc", 0, 4), (0, 0));
        assert_eq!(find_column("a\tbc", 4, 4), (2, 4));
        assert_eq!(find_column("a\tbc", 5, 4), (3, 5));
        assert_eq!(find_column("a\tbc", 10, 4), (4, 6));

        assert_eq!(count_column("日本a", 4), 5);
        assert_eq!(find_column("日本a", 2, 4), (1, 2));
        assert_eq!(find_column("日本a", 4, 4), (2, 4));
    }
}
