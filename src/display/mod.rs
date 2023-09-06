//! Talk UIs

pub mod terminal;

/// Talk Display trait
pub trait TalkDisplay {
    /// Show the UI.
    fn show(&self);

    /// Hide the UI.
    fn hide(&self);

    /// Update the UI.
    fn update(&self, text: &str);
}
