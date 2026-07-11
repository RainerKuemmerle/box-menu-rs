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
