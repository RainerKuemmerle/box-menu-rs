use freedesktop_icons::lookup;
use std::{path::PathBuf, process::Command, sync::OnceLock};

static VALUE: OnceLock<String> = OnceLock::new();

pub fn theme() -> &'static String {
    VALUE.get_or_init(detect_theme)
}

pub fn set_theme(theme: String) {
    let _ = VALUE.set(theme);
}

fn detect_theme() -> String {
    if let Ok(output) = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "icon-theme"])
        .output()
        && output.status.success()
    {
        let theme = String::from_utf8_lossy(&output.stdout)
            .trim()
            .trim_matches('"')
            .trim_matches('"')
            .trim_matches('\'')
            .to_string();
        if !theme.is_empty() {
            return theme;
        }
    }
    "hicolor".into()
}

pub fn lookup_icon(name: &str) -> Option<PathBuf> {
    lookup(name)
        .with_theme(theme().as_str())
        .with_cache()
        .find()
}
