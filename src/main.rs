use clap::Parser;
use freedesktop_desktop_entry::{DesktopEntry, desktop_entries, get_languages_from_env};

mod cli;
mod config;
mod escape;
mod icon;
mod list;
mod menu;
mod visibility;

use crate::cli::CliOptions;
use crate::config::load_config;
use crate::icon::lookup_icon;
use crate::list::list_programs;
use crate::menu::Entry;
use crate::visibility::{
    current_desktop_environment, parse_current_desktop, visibility_exclusion_reason,
};

const OPENBOX_XMLNS: &str = "http://openbox.org/";
const OPENBOX_XSI: &str = "http://www.w3.org/2001/XMLSchema-instance";

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {}", err);
        std::process::exit(1);
    }
}

fn make_entry(entry: &DesktopEntry, locales: &[String]) -> Entry {
    Entry {
        label: escape::escape(entry.full_name(locales).unwrap_or_default()).to_string(),
        exec: entry.exec().unwrap_or_default().to_string(),
        icon: entry.icon().and_then(lookup_icon),
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
            action,
        );
        return Ok(());
    }

    let mut excluded_entries = Vec::new();
    let entries: Vec<&DesktopEntry> = all_entries
        .iter()
        .filter(|x| x.categories().is_some())
        .filter(|x| {
            if !cfg.options.visibility_filter {
                return true;
            }

            if let Some(reason) = visibility_exclusion_reason(x, current_desktop_parsed.as_ref()) {
                let label = x.full_name(&locales).unwrap_or_default().to_string();
                excluded_entries.push((label, reason));
                false
            } else {
                true
            }
        })
        .collect();

    let mut root = cfg.empty_tree();
    for entry in entries {
        let mapped_categories: Vec<String> = entry
            .categories()
            .unwrap()
            .into_iter()
            .filter(|k| !k.is_empty())
            .filter(|k| cfg.category_map.contains_key(&k[..]))
            .map(|k| k.to_string())
            .collect();

        if mapped_categories.is_empty() {
            continue;
        }

        let menu_entry = make_entry(entry, &locales);

        if cfg.options.category_priority {
            let entries_category = mapped_categories
                .into_iter()
                .map(|c| {
                    let c_str: &str = c.as_ref();
                    let mapped_category = cfg.category_map.get(c_str).unwrap();
                    let output_name = mapped_category.output.as_deref().unwrap_or(c_str);
                    let priority = mapped_category.priority.unwrap_or(0);
                    (priority, output_name.to_string())
                })
                .max_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)))
                .map(|(_, output_name)| output_name)
                .unwrap();

            root.insert(&entries_category, menu_entry);
        } else {
            // Insert into every matching mapped category by default.
            for c in mapped_categories {
                let c_str: &str = c.as_ref();
                let mapped_category = cfg.category_map.get(c_str).unwrap();
                let output_name = mapped_category
                    .output
                    .clone()
                    .unwrap_or_else(|| c.to_string());
                root.insert(&output_name, menu_entry.clone());
            }
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
