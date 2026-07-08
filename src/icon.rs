use freedesktop_icons::lookup;
use std::{path::PathBuf, process::Command, sync::OnceLock};

pub fn theme() -> &'static String {
    static VALUE: OnceLock<String> = OnceLock::new();
    VALUE.get_or_init(|| {
        if let Ok(output) = Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "icon-theme"])
            .output()
        {
            if output.status.success() {
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
        } else {
            "hicolor".into()
        }
    })
}

pub fn lookup_icon(name: &str) -> Option<PathBuf> {
    lookup(name)
        .with_theme(theme().as_str())
        .with_cache()
        .find()
}
