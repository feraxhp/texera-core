use std::any::TypeId;
use std::sync::Arc;
use std::any::Any;

use crate::AnySync;


/// Marker trait for typed metadata values that can be attached to a [`Transaction`](crate::Transaction)
/// via [`Annotation`].
///
/// # Examples
///
/// ```rust
/// use texera_core::{Annotation, AnnotationValue, State, tr};
///
/// #[derive(Clone, Debug, PartialEq, Eq)]
/// struct HistoryGroup(u32);
/// impl AnnotationValue for HistoryGroup {}
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let res = State::from("abc").update(tr!({
///     annotations: [Annotation::new(HistoryGroup(7))],
/// }))?;
/// assert_eq!(res.annotation::<HistoryGroup>(), Some(&HistoryGroup(7)));
/// # Ok(())
/// # }
/// ```
pub trait AnnotationValue: Any + Send + Sync {}

/// Built-in [`AnnotationValue`] representing a hierarchical user event category
/// (e.g. `"input.type"`, `"delete.backward"`, `"select.pointer"`).
///
/// Attached automatically when [`TransactionSpec::with_user_event`](crate::TransactionSpec::with_user_event)
/// or `tr!({ user_event: "..." })` is used, and queried via
/// [`Transaction::is_user_event`](crate::Transaction::is_user_event) or
/// [`TransactionResult::is_user_event`](crate::TransactionResult::is_user_event).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserEvent(pub String);
impl AnnotationValue for UserEvent {}

impl UserEvent {
    /// Checks whether this user event matches `event` either exactly or as a dot-separated prefix
    /// (e.g. `"input"` matches `"input"` and `"input.type"`, but not `"input_other"`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use texera_core::UserEvent;
    ///
    /// let ev = UserEvent("input.type.compose".into());
    /// assert!(ev.matches("input"));
    /// assert!(ev.matches("input.type"));
    /// assert!(ev.matches("input.type.compose"));
    /// assert!(!ev.matches("input.t"));
    /// ```
    #[inline]
    pub fn matches(&self, event: &str) -> bool {
        let raw = self.0.strip_prefix("User.Event.").unwrap_or(&self.0);
        raw == event
            || (raw.len() > event.len()
                && raw.starts_with(event)
                && raw.as_bytes().get(event.len()) == Some(&b'.'))
    }
}

/// Type-erased metadata tag attached to a [`Transaction`](crate::Transaction).
///
/// Annotations describe *why* or *how* a transaction occurred (such as [`UserEvent`], undo/redo
/// markers, or collaboration remote origin tags) without directly mutating the document.
///
/// # Examples
///
/// ```rust
/// use texera_core::{Annotation, AnnotationValue};
///
/// #[derive(Clone, Debug, PartialEq, Eq)]
/// struct RemoteOrigin(&'static str);
/// impl AnnotationValue for RemoteOrigin {}
///
/// let ann = Annotation::new(RemoteOrigin("peer-42"));
/// assert!(ann.is::<RemoteOrigin>());
/// assert_eq!(ann.get::<RemoteOrigin>(), Some(&RemoteOrigin("peer-42")));
/// ```
#[derive(Clone, Debug)]
pub struct Annotation {
    type_id: TypeId,
    value: Arc<AnySync>,
}

impl Annotation {
    /// Wraps any [`AnnotationValue`] into a type-erased [`Annotation`].
    pub fn new<A: AnnotationValue>(value: A) -> Self {
        Self { 
            type_id: value.type_id(),
            value: Arc::new(value), 
        }
    }
    
    /// Creates a [`UserEvent`] annotation from a hierarchical event name (such as `"input.type"`).
    pub fn user_event(event: impl Into<String>) -> Self {
        Self::new(UserEvent(event.into()))
    }
    
    /// Returns `true` if this annotation holds a value of type `A`.
    #[inline]
    pub fn is<A: AnnotationValue>(&self) -> bool { self.type_id == TypeId::of::<A>() }
    
    /// Downcasts and returns a reference to the underlying `A` if the type matches.
    #[inline]
    pub fn get<A: AnnotationValue>(&self) -> Option<&A> {
        match self.type_id == TypeId::of::<A>() {
            true => self.value.downcast_ref::<A>(),
            false => None,
        }
    }
}
