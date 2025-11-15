use freedesktop_desktop_entry::{desktop_entries, get_languages_from_env};
use freedesktop_icons::{default_theme_gtk, lookup};
use itertools::Itertools;
use lazy_static::lazy_static;
use quick_xml::escape::escape;
use std::fmt;
use std::sync::OnceLock;
use std::{collections::HashMap, collections::HashSet, path::PathBuf};

#[derive(Eq, Hash, PartialEq, PartialOrd, Ord)]
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

fn theme() -> &'static String {
    static VALUE: OnceLock<String> = OnceLock::new();
    VALUE.get_or_init(|| {
        if let Some(theme) = default_theme_gtk() {
            theme
        } else {
            "hicolor".to_string()
        }
    })
}

fn lookup_icon(name: &str) -> Option<PathBuf> {
    lookup(name)
        .with_theme(theme().as_str())
        .with_cache()
        .find()
}

lazy_static! {
    static ref CATEGORIES_MAP: HashMap<String, String> = {
        let mut m = HashMap::new();
        m.insert("AudioVideo".to_string(), "Multimedia".to_string());
        m.insert("Audio".to_string(), "Multimedia".to_string());
        m.insert("Video".to_string(), "Multimedia".to_string());
        m.insert("Development".to_string(), "Development".to_string());
        m.insert("Education".to_string(), "Education".to_string());
        m.insert("Game".to_string(), "Games".to_string());
        m.insert("Graphics".to_string(), "Graphics".to_string());
        m.insert("Network".to_string(), "Internet".to_string());
        m.insert("Office".to_string(), "Office".to_string());
        m.insert("Science".to_string(), "Science".to_string());
        m.insert("Settings".to_string(), "Settings".to_string());
        m.insert("System".to_string(), "System".to_string());
        m.insert("Utility".to_string(), "Utility".to_string());
        m
    };
}

fn empty_hash() -> HashMap<String, HashSet<Entry>> {
    let mut hm = HashMap::new();
    for c in CATEGORIES_MAP.values().into_iter() {
        hm.insert(c.to_string(), HashSet::new());
    }
    hm
}

fn main() {
    let locales = get_languages_from_env();
    let entries = desktop_entries(&locales);

    let mut menu_entries = empty_hash();
    for entry in entries.iter().filter(|x| x.categories().is_some()) {
        for c in entry
            .categories()
            .unwrap()
            .into_iter()
            .filter(|&k| CATEGORIES_MAP.contains_key(k))
        {
            let mapped_category = CATEGORIES_MAP.get(c).unwrap();
            if let Some(v) = menu_entries.get_mut(mapped_category) {
                v.insert(Entry {
                    label: escape(entry.full_name(&locales).unwrap_or_default()).to_string(),
                    exec: entry.exec().unwrap_or_default().to_string(),
                    icon: if let Some(ei) = entry.icon() {
                        lookup_icon(ei)
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
        let category_icon_name = format!("applications-{}", category.to_lowercase());
        let icon_str = if let Some(icon_path) = lookup_icon(&category_icon_name) {
            format!(" icon=\"{}\"", icon_path.display())
        } else {
            "".to_string()
        };
        println!("<menu id=\"boxmenu-{category}\" label=\"{category}\" {icon_str}>");
        for e in entries_in_cat.iter().sorted() {
            println!("{e}");
        }
        println!("</menu>");
    }
    println!("</openbox_menu>");
}
