//! Access the clipboard.

/// A buffer for short-term storage and transfer within and between
/// applications.
pub trait Clipboard {
    /// Reads the current content of the [`Clipboard`] as text.
    fn read(&self) -> Option<String>;

    /// Writes the given text contents to the [`Clipboard`].
    fn write(&mut self, contents: String);
}

/// A null implementation of the [`Clipboard`] trait.
#[derive(Debug, Clone, Copy)]
pub struct Null;

impl Clipboard for Null {
    fn read(&self) -> Option<String> {
        None
    }

    fn write(&mut self, _contents: String) {}
}

pub enum Action<T> {
    Read(Box<dyn Fn(Option<String>) -> T>),
    Write(String),
}

impl<T> Action<T> {
    pub fn map<A>(self, f: impl Fn(T) -> A + 'static + Send + Sync) -> Action<A>
    where
        T: 'static,
    {
        match self {
            Self::Read(o) => Action::Read(Box::new(move |s| f(o(s)))),
            Self::Write(content) => Action::Write(content),
        }
    }
}
