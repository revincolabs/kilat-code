//! Language of the text the backend shows: errors, notifications, the
//! statusline, and the messages Kilat types to agents (so they answer in the
//! same language). The UI decides ("auto" follows the system) and tells us
//! with the `set_ui_lang` command.

use std::sync::atomic::{AtomicBool, Ordering};

static EN: AtomicBool = AtomicBool::new(false);

pub fn set(lang: &str) {
    EN.store(lang == "en", Ordering::Relaxed);
}

pub fn en() -> bool {
    EN.load(Ordering::Relaxed)
}

/// The Indonesian or the English text, whichever the UI uses.
pub fn tr(id: &'static str, en: &'static str) -> &'static str {
    if self::en() {
        en
    } else {
        id
    }
}

/// `format!` with an Indonesian and an English template.
#[macro_export]
macro_rules! trf {
    ($id:literal, $en:literal $(, $arg:expr)* $(,)?) => {
        if $crate::i18n::en() { format!($en $(, $arg)*) } else { format!($id $(, $arg)*) }
    };
}

/// Usage window labels are stored in Indonesian ("5 jam", "7 hari (Opus)").
pub fn window(label: &str) -> String {
    window_in(en(), label)
}

fn window_in(en: bool, label: &str) -> String {
    if en {
        label.replace(" jam", "h").replace(" hari", "d")
    } else {
        label.to_string()
    }
}

#[cfg(test)]
mod tests {
    // Tests run in parallel and others expect Indonesian, so never flip the global here.
    #[test]
    fn window_labels() {
        assert_eq!(super::window_in(true, "5 jam"), "5h");
        assert_eq!(super::window_in(true, "7 hari (Opus)"), "7d (Opus)");
        assert_eq!(super::window_in(false, "5 jam"), "5 jam");
        assert_eq!(crate::trf!("{} jam", "{} h", 3), "3 jam");
    }
}
