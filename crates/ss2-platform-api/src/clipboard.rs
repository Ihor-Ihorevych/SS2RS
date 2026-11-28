//! The system clipboard, as far as the app needs it: reading text to paste into a text field (T10-035,
//! RFC 0003 amendment 1). The real one is `ss2_platform::clipboard::SystemClipboard`; [`MockClipboard`]
//! stands in for it in tests and wherever there is no clipboard.

/// Reads text from a clipboard.
///
/// ```
/// use ss2_platform_api::{Clipboard, MockClipboard};
///
/// let mut clipboard = MockClipboard::with_text("C:\\Games\\Serious Sam 2");
/// assert_eq!(clipboard.text().as_deref(), Some("C:\\Games\\Serious Sam 2"));
/// assert_eq!(MockClipboard::default().text(), None);
/// ```
pub trait Clipboard {
    /// The text on the clipboard, as it is (new lines included); `None` when it holds no text or cannot be read.
    fn text(&mut self) -> Option<String>;
}

/// A clipboard that holds a fixed text, or nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MockClipboard {
    /// What [`Clipboard::text`] returns.
    pub text: Option<String>,
}

impl MockClipboard {
    /// A clipboard holding `text`.
    pub fn with_text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
        }
    }
}

impl Clipboard for MockClipboard {
    fn text(&mut self) -> Option<String> {
        self.text.clone()
    }
}
