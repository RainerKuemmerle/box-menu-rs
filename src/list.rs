use crate::{
    config::Config,
    icon::{lookup_icon, resolve_icon},
    visibility::visibility_exclusion_reason,
};
use freedesktop_desktop_entry::DesktopEntry;
use std::collections::HashSet;

pub fn list_programs(
    entries: &[DesktopEntry],
    locales: &[String],
    config: &Config,
    current_desktop: Option<&HashSet<String>>,
    program_name: Option<&str>,
    action: crate::cli::ListAction,
) {
    let program_name_filter = program_name.map(|name| name.to_lowercase());

    let mut entries: Vec<_> = entries
        .iter()
        .filter(|entry| entry.categories().is_some())
        .filter(|entry| match action {
            crate::cli::ListAction::All => true,
            crate::cli::ListAction::MissingIcons => {
                let icon_field = entry.icon().unwrap_or_default();
                !icon_field.is_empty() && entry.icon().and_then(lookup_icon).is_none()
            }
            crate::cli::ListAction::Excluded => {
                visibility_exclusion_reason(entry, current_desktop).is_some()
            }
            crate::cli::ListAction::Program => {
                if let Some(filter_name) = program_name_filter.as_deref() {
                    entry
                        .full_name(locales)
                        .unwrap_or_default()
                        .to_lowercase()
                        .contains(filter_name)
                } else {
                    false
                }
            }
        })
        .collect();
    entries.sort_by_key(|entry| entry.full_name(locales).unwrap_or_default());

    match action {
        crate::cli::ListAction::All => println!("Discovered desktop entries:"),
        crate::cli::ListAction::MissingIcons => {
            println!("Desktop entries with missing entry icon lookup:");
        }
        crate::cli::ListAction::Excluded => println!("Hidden/excluded desktop entries:"),
        crate::cli::ListAction::Program => {
            if let Some(name) = program_name {
                println!("Desktop entries matching Name: {}", name);
            } else {
                println!("Desktop entries matching Name:");
            }
        }
    }

    if entries.is_empty() {
        println!("<none>");
        return;
    }

    for entry in entries {
        let label = entry.full_name(locales).unwrap_or_default();
        let desktop_file_path = entry.path.to_string_lossy();
        let exec = entry.exec().unwrap_or_default();
        let icon_field = entry.icon().unwrap_or_default();
        let entry_icon_path = entry.icon().and_then(lookup_icon);
        let visibility_reason = visibility_exclusion_reason(entry, current_desktop);
        let excluded_by_filter = visibility_reason.is_some() && config.options.visibility_filter;

        println!("\nProgram: {}", label);
        println!("  Desktop file: {}", desktop_file_path);
        println!("  Exec: {}", exec);
        println!(
            "  Icon field: {}",
            if icon_field.is_empty() {
                "<none>"
            } else {
                &icon_field
            }
        );
        match entry_icon_path {
            Some(path) => println!("  Resolved entry icon: {}", path.display()),
            None if icon_field.is_empty() => {
                println!("  Entry icon is not defined in the desktop file.")
            }
            None => println!("  Entry icon lookup failed for '{}'.", icon_field),
        }
        if let Some(reason) = visibility_reason {
            if config.options.visibility_filter {
                println!("  Visibility: excluded ({})", reason);
            } else {
                println!(
                    "  Visibility: would be excluded ({}) but filtering is disabled",
                    reason
                );
            }
        } else {
            println!("  Visibility: included");
        }

        let categories = entry.categories().unwrap_or_default();
        let categories: Vec<_> = categories.into_iter().filter(|c| !c.is_empty()).collect();
        if categories.is_empty() {
            println!("  Categories: <none>");
            continue;
        }

        let mut mapped_categories = Vec::new();
        for category in &categories {
            if let Some(mapped_category) = config.category_map.get(*category) {
                mapped_categories.push((category, mapped_category));
            }
        }

        if config.options.category_priority && !mapped_categories.is_empty() {
            let chosen = mapped_categories.iter().max_by(
                |(category_a, mapped_a), (category_b, mapped_b)| {
                    let priority_a = mapped_a.priority.unwrap_or(0);
                    let priority_b = mapped_b.priority.unwrap_or(0);
                    priority_a.cmp(&priority_b).then(
                        mapped_a
                            .output
                            .as_deref()
                            .unwrap_or(category_a.as_ref())
                            .cmp(mapped_b.output.as_deref().unwrap_or(category_b.as_ref())),
                    )
                },
            );

            if let Some((category, mapped_category)) = chosen {
                let output_name = mapped_category
                    .output
                    .as_deref()
                    .unwrap_or(category.as_ref());
                let priority = mapped_category.priority.unwrap_or(0);
                println!(
                    "  Category priority enabled: selected '{}' (priority {})",
                    output_name, priority
                );
            }
        }

        for category in categories {
            if let Some(mapped_category) = config.category_map.get(category) {
                let output_name = mapped_category.output.as_deref().unwrap_or(category);
                let category_icon_name = config.icon_for_category(output_name);
                let category_icon_path = resolve_icon(&category_icon_name);

                println!("  Category: {}", category);
                println!("    Mapped output: {}", output_name);
                if let Some(priority) = mapped_category.priority {
                    println!("    Priority: {}", priority);
                } else {
                    println!("    Priority: <default>");
                }
                println!("    Category icon: {}", category_icon_name);
                match category_icon_path {
                    Some(path) => println!("    Resolved category icon: {}", path.display()),
                    None => println!("    Category icon lookup failed."),
                }
            } else {
                println!("  Category: {} (not mapped)", category);
            }
        }

        if excluded_by_filter {
            println!(
                "  Note: this entry would be excluded from XML output by visibility filtering."
            );
        }
    }
}
