use clap::Parser;
use freedesktop_desktop_entry::{DesktopEntry, desktop_entries, get_languages_from_env};

mod cli;
mod config;
mod escape;
mod icon;
mod menu;
mod visibility;

use crate::cli::CliOptions;
use crate::config::{load_config, Config};
use crate::icon::lookup_icon;
use crate::menu::Entry;
use crate::visibility::{
    current_desktop_environment, parse_current_desktop, visibility_exclusion_reason,
};
use std::collections::HashSet;

const OPENBOX_XMLNS: &str = "http://openbox.org/";
const OPENBOX_XSI: &str = "http://www.w3.org/2001/XMLSchema-instance";

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli_options = CliOptions::parse();
    let cfg = load_config(cli_options.config_file())?;

    if let Some(theme) = cfg.options.icon_theme.clone() {
        crate::icon::set_theme(theme);
    }

    let locales = get_languages_from_env();
    let current_desktop = current_desktop_environment();
    let current_desktop_parsed = current_desktop.as_deref().map(parse_current_desktop);
    let all_entries: Vec<DesktopEntry> = desktop_entries(&locales).into_iter().collect();
    let program_name = cli_options.program_name();
    let program_name_filter = program_name.map(|name| name.to_lowercase());

    if let Some(action) = cli_options.list_action() {
        if matches!(action, crate::cli::ListAction::Program) && program_name.is_none() {
            return Err(Box::new(clap::Error::raw(
                clap::error::ErrorKind::MissingRequiredArgument,
                "NAME is required when --list program is used",
            )));
        }
        if !matches!(action, crate::cli::ListAction::Program) && program_name.is_some() {
            return Err(Box::new(clap::Error::raw(
                clap::error::ErrorKind::ArgumentConflict,
                "NAME can only be used with --list program",
            )));
        }

        list_programs(
            &all_entries,
            &locales,
            &cfg,
            current_desktop_parsed.as_ref(),
            program_name,
            program_name_filter.as_deref(),
            action,
        );
        return Ok(());
    }

    let mut excluded_entries = Vec::new();
    let entries: Vec<&DesktopEntry> = all_entries
        .iter()
        .filter(|x| x.categories().is_some())
        .filter(|x| {
            if cfg.options.visibility_filter {
                if let Some(reason) =
                    visibility_exclusion_reason(x, current_desktop_parsed.as_ref())
                {
                    let label = x.full_name(&locales).unwrap_or_default().to_string();
                    excluded_entries.push((label, reason));
                    false
                } else {
                    true
                }
            } else {
                true
            }
        })
        .collect();

    let mut root = cfg.empty_tree();
    for entry in entries {
        for c in entry
            .categories()
            .unwrap()
            .into_iter()
            .filter(|k| !k.is_empty())
            .filter(|&k| cfg.category_map.contains_key(k))
        {
            let mapped_category = cfg.category_map.get(c).unwrap();
            let output_name = mapped_category.output.as_ref().map_or(c, |v| v);
            root.insert(
                output_name,
                Entry {
                    label: escape::escape(entry.full_name(&locales).unwrap_or_default())
                        .to_string(),
                    exec: entry.exec().unwrap_or_default().to_string(),
                    icon: entry.icon().and_then(lookup_icon),
                },
            );
        }
    }

    println!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    println!(
        "<openbox_menu xmlns=\"{}\" xmlns:xsi=\"{}\" xsi:schemaLocation=\"{}\" >",
        OPENBOX_XMLNS, OPENBOX_XSI, OPENBOX_XMLNS
    );
    root.print(&cfg, "");
    println!("</openbox_menu>");

    if !excluded_entries.is_empty() {
        println!("<!-- Excluded entries (visibility filtering):");
        for (label, reason) in excluded_entries {
            let comment_line = format!("  {} ({})", label, reason).replace("--", "—");
            println!("{}", comment_line);
        }
        println!("-->");
    }

    Ok(())
}

fn list_programs(
    entries: &[DesktopEntry],
    locales: &[String],
    config: &Config,
    current_desktop: Option<&HashSet<String>>,
    program_name: Option<&str>,
    program_name_filter: Option<&str>,
    action: crate::cli::ListAction,
) {
    let mut entries: Vec<_> = entries
        .iter()
        .filter(|entry| entry.categories().is_some())
        .filter(|entry| match action {
            crate::cli::ListAction::All => true,
            crate::cli::ListAction::MissingIcons => {
                let icon_field = entry.icon().unwrap_or_default();
                !icon_field.is_empty() && entry.icon().and_then(lookup_icon).is_none()
            }
            crate::cli::ListAction::Excluded => visibility_exclusion_reason(entry, current_desktop).is_some(),
            crate::cli::ListAction::Program => {
                if let Some(filter_name) = program_name_filter {
                    entry.full_name(locales).unwrap_or_default().to_lowercase() == filter_name
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
        println!("  Icon field: {}", if icon_field.is_empty() { "<none>" } else { &icon_field });
        match entry_icon_path {
            Some(path) => println!("  Resolved entry icon: {}", path.display()),
            None if icon_field.is_empty() => println!("  Entry icon is not defined in the desktop file."),
            None => println!("  Entry icon lookup failed for '{}'.", icon_field),
        }
        if let Some(reason) = visibility_reason {
            if config.options.visibility_filter {
                println!("  Visibility: excluded ({})", reason);
            } else {
                println!("  Visibility: would be excluded ({}) but filtering is disabled", reason);
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

        for category in categories {
            match config.category_map.get(&category[..]) {
                Some(mapped_category) => {
                    let output_name = mapped_category.output.as_deref().unwrap_or(&category);
                    let category_icon_name = config.icon_for_category(output_name);
                    let category_icon_path = lookup_icon(&category_icon_name);

                    println!("  Category: {}", category);
                    println!("    Mapped output: {}", output_name);
                    println!("    Category icon: {}", category_icon_name);
                    match category_icon_path {
                        Some(path) => println!("    Resolved category icon: {}", path.display()),
                        None => println!("    Category icon lookup failed."),
                    }
                }
                None => println!("  Category: {} (not mapped)", category),
            }
        }

        if excluded_by_filter {
            println!("  Note: this entry would be excluded from XML output by visibility filtering.");
        }
    }
}
