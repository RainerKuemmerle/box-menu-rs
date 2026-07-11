use clap::Parser;
use freedesktop_desktop_entry::DesktopEntry;
use std::{collections::HashSet, path::PathBuf};

use crate::{config::Config, icon::lookup_icon, visibility::visibility_exclusion_reason};

#[derive(Debug, Parser)]
#[command(author, version, about = "Generate an Openbox-compatible application menu", long_about = None)]
pub struct CliOptions {
    #[arg(
        long = "debug-program",
        help = "Inspect icon lookup for a given desktop entry Name",
    )]
    program_name: Option<String>,

    #[arg(
        long = "config-file",
        value_name = "PATH",
        help = "Load configuration from a specific YAML file instead of the default XDG config",
    )]
    config_file: Option<PathBuf>,

    #[arg(
        long = "list-programs",
        help = "List discovered desktop entries and their mapped output categories instead of generating XML",
    )]
    list_programs: bool,
}

impl CliOptions {
    pub fn program_name(&self) -> Option<&str> {
        self.program_name.as_deref()
    }

    pub fn config_file(&self) -> Option<&PathBuf> {
        self.config_file.as_ref()
    }

    pub fn list_programs(&self) -> bool {
        self.list_programs
    }
}

pub fn debug_program_icon_resolution(
    entries: &[DesktopEntry],
    locales: &[String],
    config: &Config,
    cli_options: &CliOptions,
    current_desktop: Option<&HashSet<String>>,
) {
    let name = match cli_options.program_name() {
        Some(name) => name,
        None => return,
    };

    eprintln!("DEBUG: inspecting program '{}'", name);
    eprintln!("DEBUG: icon theme = {}", crate::icon::theme());
    eprintln!(
        "DEBUG: visibility filtering = {}",
        if config.options.visibility_filter {
            "enabled"
        } else {
            "disabled"
        }
    );
    eprintln!(
        "DEBUG: current desktop = {}",
        current_desktop
            .map(|values| values.iter().cloned().collect::<Vec<_>>().join(", "))
            .unwrap_or_else(|| "<unknown>".into())
    );

    let normalized_name = name.to_lowercase();
    let matching: Vec<_> = entries
        .iter()
        .filter(|entry| {
            entry.full_name(locales).unwrap_or_default().to_lowercase() == normalized_name
        })
        .collect();

    if matching.is_empty() {
        eprintln!("DEBUG: no desktop entry matched the Name '{}'.", name);
        return;
    }

    for (index, entry) in matching.into_iter().enumerate() {
        let label = entry.full_name(locales).unwrap_or_default();
        let desktop_file_path = entry.path.to_string_lossy();
        let exec = entry.exec().unwrap_or_default();
        let icon_field = entry.icon().unwrap_or_default();
        let entry_icon_path = entry.icon().and_then(lookup_icon);
        let visibility_reason = visibility_exclusion_reason(entry, current_desktop);

        eprintln!("DEBUG: match #{}", index + 1);

        eprintln!("  Name: {}", label);
        eprintln!("  Desktop file: {}", desktop_file_path);
        eprintln!("  Exec: {}", exec);
        eprintln!(
            "  Desktop icon field: {}",
            if icon_field.is_empty() {
                "<none>"
            } else {
                &icon_field
            }
        );
        eprintln!(
            "  Exclusion reason: {}",
            visibility_reason.unwrap_or_else(|| "<none>".into())
        );

        match entry_icon_path {
            Some(path) => eprintln!("  Resolved entry icon: {}", path.display()),
            None if icon_field.is_empty() => {
                eprintln!("  Entry icon is not defined in the desktop file.")
            }
            None => eprintln!("  Entry icon lookup failed for '{}'.", icon_field),
        }

        let categories = entry.categories().unwrap_or_default();
        let categories: Vec<_> = categories.into_iter().filter(|c| !c.is_empty()).collect();
        if categories.is_empty() {
            eprintln!("  Categories: <none>");
            continue;
        }

        for category in categories {
            eprintln!("  Category: {}", category);
            let category_name = &category[..];
            match config.category_map.get(category_name) {
                Some(mapped_category) => {
                    let output_name = mapped_category.output.as_deref().unwrap_or(category_name);
                    let category_icon_name = config.icon_for_category(output_name);
                    let category_icon_path = lookup_icon(&category_icon_name);

                    eprintln!("    Output path: {}", output_name);
                    eprintln!("    Category icon name: {}", category_icon_name);

                    match category_icon_path {
                        Some(path) => eprintln!("    Resolved category icon: {}", path.display()),
                        None => eprintln!(
                            "    Category icon lookup failed for '{}'.",
                            category_icon_name
                        ),
                    }
                }
                None => {
                    eprintln!("    Category is not included in category_map and will be ignored.");
                }
            }
        }
    }
    eprintln!("DEBUG: inspection complete.");
}
