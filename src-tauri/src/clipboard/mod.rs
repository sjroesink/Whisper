use anyhow::Result;
use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::thread;
use std::time::Duration;

/// Copy text to clipboard and simulate a paste keystroke.
pub fn paste_text(text: &str) -> Result<()> {
    // Set text to clipboard
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text)?;

    // Small delay to ensure clipboard is ready
    thread::sleep(Duration::from_millis(100));

    // Simulate paste keystroke
    let mut enigo = Enigo::new(&Settings::default())?;

    #[cfg(target_os = "macos")]
    {
        enigo.key(Key::Meta, Direction::Press)?;
        enigo.key(Key::Unicode('v'), Direction::Click)?;
        enigo.key(Key::Meta, Direction::Release)?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        enigo.key(Key::Control, Direction::Press)?;
        enigo.key(Key::Unicode('v'), Direction::Click)?;
        enigo.key(Key::Control, Direction::Release)?;
    }

    Ok(())
}
