use freedesktop_desktop_entry::DesktopEntry;
use std::{collections::HashSet, env};

pub fn current_desktop_environment() -> Option<String> {
    env::var("XDG_CURRENT_DESKTOP")
        .ok()
        .filter(|val| !val.trim().is_empty())
}

fn normalize_desktop_token(token: &str) -> Option<String> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_lowercase())
    }
}

pub fn parse_current_desktop(current_desktop: &str) -> HashSet<String> {
    current_desktop
        .split(':')
        .flat_map(str::split_ascii_whitespace)
        .filter_map(normalize_desktop_token)
        .collect()
}

fn normalize_entry_desktop_tokens(tokens: &[&str]) -> Vec<String> {
    tokens
        .iter()
        .filter_map(|token| normalize_desktop_token(token))
        .collect()
}

pub fn visibility_exclusion_reason(
    entry: &DesktopEntry,
    current_desktop: Option<&HashSet<String>>,
) -> Option<String> {
    if entry.hidden() {
        return Some("Hidden=true".into());
    }

    if entry.no_display() {
        return Some("NoDisplay=true".into());
    }

    if let Some(only_show) = entry.only_show_in()
        && let Some(current) = current_desktop
    {
        let normalized_allowed = normalize_entry_desktop_tokens(&only_show);
        if !normalized_allowed.is_empty()
            && normalized_allowed
                .iter()
                .all(|allowed| !current.contains(allowed.as_str()))
        {
            return Some(format!("OnlyShowIn={:?}", normalized_allowed));
        }
    }

    if let Some(not_show) = entry.not_show_in()
        && let Some(current) = current_desktop
    {
        let normalized_blocked = normalize_entry_desktop_tokens(&not_show);
        if let Some(blocked) = normalized_blocked
            .iter()
            .find(|blocked| current.contains(blocked.as_str()))
        {
            return Some(format!("NotShowIn={}", blocked));
        }
    }

    None
}
