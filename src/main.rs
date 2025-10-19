use freedesktop_desktop_entry::{desktop_entries, get_languages_from_env};
use freedesktop_icons::lookup;
use quick_xml::escape::escape;
use std::{collections::HashMap, path::PathBuf};

struct Entry {
    label: String,
    exec: String,
    icon: Option<String>,
}

fn default_categories() -> Vec<String> {
    let categories: Vec<String> = vec![
        "AudioVideo".to_string(),
        "Audio".to_string(),
        "Video".to_string(),
        "Development".to_string(),
        "Education".to_string(),
        "Game".to_string(),
        "Graphics".to_string(),
        "Network".to_string(),
        "Office".to_string(),
        "Science".to_string(),
        "Settings".to_string(),
        "System".to_string(),
        "Utility".to_string(),
    ];
    categories
}

fn empty_hash() -> HashMap<String, Vec<Entry>> {
    let categories = default_categories();
    let mut hm = HashMap::new();
    for c in categories.into_iter() {
        hm.insert(c, Vec::new());
    }
    hm
}

fn print_without_icon(e: &Entry) {
    println!(
        "<item label=\"{0}\"><action name=\"Execute\"><command>{1}</command></action></item>",
        escape(e.label.as_str()),
        e.exec
    )
}

fn print_with_icon(e: &Entry, icon: Option<PathBuf>) {
    match icon {
        None => print_without_icon(e),
        Some(path) => println!(
            "<item label=\"{0}\" icon=\"{2}\"><action name=\"Execute\"><command>{1}</command></action></item>",
            escape(e.label.as_str()),
            e.exec,
            path.display()
        ),
    }
}

fn main() {
    let locales = get_languages_from_env();
    let entries = desktop_entries(&locales);

    let mut menu_entries = empty_hash();
    for entry in entries.iter().filter(|x| x.categories().is_some()) {
        for c in entry.categories().unwrap() {
            if let Some(v) = menu_entries.get_mut(c) {
                v.push(Entry {
                    label: entry.full_name(&locales).unwrap_or_default().to_string(),
                    exec: entry.exec().unwrap_or_default().to_string(),
                    icon: entry.icon().map(str::to_string),
                });
            }
        }
    }

    println!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    println!(
        "<openbox_menu xmlns=\"http://openbox.org/\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"http://openbox.org/\" >"
    );

    for category in default_categories() {
        let entries_in_cat = menu_entries.get(&category).unwrap();
        if entries_in_cat.is_empty() {
            continue;
        }
        println!("<menu id=\"boxmenu-{category}\" label=\"{category}\" >");
        for e in entries_in_cat {
            match &e.icon {
                None => print_without_icon(e),
                Some(icon) => print_with_icon(e, lookup(icon).with_cache().find()),
            }
        }
        println!("</menu>");
    }
    println!("</openbox_menu>");
}
