use freedesktop_desktop_entry::{desktop_entries, get_languages_from_env};
use freedesktop_icons::lookup;
use quick_xml::escape::escape;
use std::fmt;
use std::{collections::HashMap, path::PathBuf};
use itertools::Itertools;

struct Entry {
    label: String,
    exec: String,
    icon: Option<PathBuf>,
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let icon_str = if let Some(icon_path) = &self.icon {
            format!(" icon=\"{}\"", icon_path.display())
        } else {
            "".to_string()
        };
        write!(
            f,
            "<item label=\"{}\"{}><action name=\"Execute\"><command>{}</command></action></item>",
            self.label, icon_str, self.exec,
        )
    }
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

fn main() {
    let locales = get_languages_from_env();
    let entries = desktop_entries(&locales);

    let mut menu_entries = empty_hash();
    for entry in entries.iter().filter(|x| x.categories().is_some()) {
        for c in entry.categories().unwrap() {
            if let Some(v) = menu_entries.get_mut(c) {
                v.push(Entry {
                    label: escape(entry.full_name(&locales).unwrap_or_default()).to_string(),
                    exec: entry.exec().unwrap_or_default().to_string(),
                    icon: if let Some(ei) = entry.icon() {
                        lookup(ei).with_cache().find()
                    } else {
                        None
                    },
                });
            }
        }
    }

    println!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    println!(
        "<openbox_menu xmlns=\"http://openbox.org/\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"http://openbox.org/\" >"
    );

    for category in menu_entries.keys().sorted() {
        let entries_in_cat = menu_entries.get(category).unwrap();
        if entries_in_cat.is_empty() {
            continue;
        }
        println!("<menu id=\"boxmenu-{category}\" label=\"{category}\" >");
        for e in entries_in_cat {
            println!("{}", e);
        }
        println!("</menu>");
    }
    println!("</openbox_menu>");
}
