use super::Extension;
use super::Priority;


impl Extension {
    /// Attaches an explicit [`Priority`] tier to this [`Extension`] (and any nested children that
    /// do not override their own priority).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::{Extension, Priority, State, TabSize};
    ///
    /// let state = State::from(Extension::from([
    ///     TabSize(2).into(),
    ///     Extension::from(TabSize(8)).with_priority(Priority::High),
    /// ]));
    /// assert_eq!(state.tab_size(), 8);
    /// ```
    pub fn with_priority(self, priority: Priority) -> Self {
        Extension::Prec(priority, Box::new(self))
    }
}
