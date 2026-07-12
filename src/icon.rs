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

pub fn resolve_icon(name_or_path: &str) -> Option<PathBuf> {
    let path = PathBuf::from(name_or_path);
    if path.is_file() {
        return Some(path);
    }
    lookup_icon(name_or_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn resolve_icon_returns_existing_file_path() {
        let temp_dir = std::env::temp_dir();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let file_path = temp_dir.join(format!("box-menu-rs-test-icon-{}.png", timestamp));

        fs::write(&file_path, b"test").expect("failed to write temp file");
        let resolved = resolve_icon(file_path.to_str().expect("invalid temp path"));
        assert_eq!(resolved.as_deref(), Some(file_path.as_path()));

        fs::remove_file(&file_path).expect("failed to remove temp file");
    }

    #[test]
    fn resolve_icon_returns_none_for_nonexistent_file_path() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let path_str = format!(
            "/tmp/box-menu-rs-nonexistent-icon-{}.doesnotexist",
            timestamp
        );

        let resolved = resolve_icon(&path_str);
        assert!(resolved.is_none());
    }
}
