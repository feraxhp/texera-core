use super::super::Text;


impl Text {
    /// Returns a new [`Text`] with the character range `[from..to]` replaced by `text`.
    ///
    /// - `from`: Start character offset (defaults to `0` if `None`).
    /// - `to`: End character offset (defaults to `self.len_chars()` if `None`).
    ///
    /// Out-of-bounds indices are clamped to the document bounds.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::Text;
    ///
    /// let doc = Text::from("hello world");
    /// let updated = doc.replace(Some(6), Some(11), Text::from("rust"));
    /// assert_eq!(updated.to_string(), "hello rust");
    /// ```
    pub fn replace(mut self, from: Option<usize>, to: Option<usize>, text: Self) -> Self {
        let from_char = from.unwrap_or(0);
        let to_char = to.unwrap_or(self.rope.len_chars());
        
        let to_char = to_char.min(self.rope.len_chars()).max(from_char);
        
        let mut tail = self.rope.split_off(from_char);
        let after = tail.split_off(to_char - from_char);
        
        self.rope.append(text.rope);
        self.rope.append(after);
        
        self
    }
    
    /// Appends another [`Text`] to the end of `self` and returns the combined document.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::Text;
    ///
    /// let doc = Text::from("foo").append(Text::from("bar"));
    /// assert_eq!(doc.to_string(), "foobar");
    /// ```
    pub fn append(mut self, text: Self) -> Self {
        self.rope.append(text.rope);
        self
    }

    /// Inserts `text` in-place at `char_idx`.
    ///
    /// If `char_idx >= self.len_chars()`, `text` is appended at the end.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::Text;
    ///
    /// let mut doc = Text::from("ac");
    /// doc.insert(1, &Text::from("b"));
    /// assert_eq!(doc.to_string(), "abc");
    /// ```
    pub fn insert(&mut self, char_idx: usize, text: &Text) {
        if char_idx >= self.rope.len_chars() {
            self.rope.append(text.rope.clone());
        } else {
            let tail = self.rope.split_off(char_idx);
            self.rope.append(text.rope.clone());
            self.rope.append(tail);
        }
    }

    /// Removes the character range `char_range` in-place.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::Text;
    ///
    /// let mut doc = Text::from("abcdef");
    /// doc.remove(1..4);
    /// assert_eq!(doc.to_string(), "aef");
    /// ```
    pub fn remove(&mut self, char_range: std::ops::Range<usize>) {
        if !char_range.is_empty() {
            self.rope.remove(char_range);
        }
    }
    
    /// Extracts a sub-document covering `[from..to]`.
    ///
    /// If `to` is `None`, slices up to the end of the document. Out-of-bounds positions are clamped.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::Text;
    ///
    /// let doc = Text::from("hello world");
    /// assert_eq!(doc.slice(0, Some(5)).to_string(), "hello");
    /// assert_eq!(doc.slice(6, None).to_string(), "world");
    /// ```
    pub fn slice(&self, from: usize, to: Option<usize>) -> Self {
        let len = self.rope.len_chars();
        
        let start = from.min(len);
        let end = to.unwrap_or(len).clamp(start, len);
        
        Text {
            rope: self.rope.slice(start..end).into(),
        }
    }
    
    /// Returns the string slice between `from` and `to`, optionally normalizing line breaks to `line_sep`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::Text;
    ///
    /// let doc = Text::from("a\nb\nc");
    /// assert_eq!(doc.slice_string(0, None, Some("\r\n".to_string())), "a\r\nb\r\nc");
    /// ```
    pub fn slice_string(&self, from: usize, to: Option<usize>, line_sep: Option<String>) -> String {
        let len = self.rope.len_chars();
        let start = from.min(len);
        let end = to.unwrap_or(len).clamp(start, len);
        
        let slice_str = self.rope.slice(start..end).to_string();
        
        match line_sep {
            Some(sep) => slice_str.replace("\r\n", "\n").replace('\n', &sep),
            None => slice_str,
        }
    }
}
