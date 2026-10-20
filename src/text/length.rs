use super::super::Text;


impl Text {
    /// Returns the total length of the document in UTF-8 bytes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::Text;
    ///
    /// let doc = Text::from("héllo");
    /// assert_eq!(doc.len_bytes(), 6);
    /// assert_eq!(doc.len_chars(), 5);
    /// ```
    pub fn len_bytes(&self) -> usize { self.rope.len_bytes() }

    /// Returns the total number of Unicode scalar values (`char`s) in the document.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::Text;
    ///
    /// let doc = Text::from("abc\ndef");
    /// assert_eq!(doc.len_chars(), 7);
    /// ```
    pub fn len_chars(&self) -> usize { self.rope.len_chars() }

    /// Returns the number of lines in the document (always at least `1`, even for an empty document).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::Text;
    ///
    /// assert_eq!(Text::new().len_lines(), 1);
    /// assert_eq!(Text::from(["a", "b", "c"]).len_lines(), 3);
    /// ```
    pub fn len_lines(&self) -> usize { self.rope.len_lines() }
}
