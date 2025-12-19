use freedesktop_desktop_entry::{DesktopEntry, desktop_entries, get_languages_from_env};
use freedesktop_icons::{default_theme_gtk, lookup};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::OnceLock;
use std::{collections::HashMap, collections::HashSet, path::PathBuf};

mod escape;

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
            "".into()
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
            "hicolor".into()
        }
    })
}

fn lookup_icon(name: &str) -> Option<PathBuf> {
    lookup(name)
        .with_theme(theme().as_str())
        .with_cache()
        .find()
}

#[derive(Serialize, Deserialize)]
struct ConfigCategory {
    output: Option<String>,
}
impl ConfigCategory {
    pub fn default(output_name: String) -> Self {
        Self {
            output: Some(output_name),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct OutputCategory {
    icon: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Config {
    category_map: HashMap<String, ConfigCategory>,
    output: Option<HashMap<String, OutputCategory>>,
}
impl Config {
    pub fn empty_hash(&self) -> HashMap<String, HashSet<Entry>> {
        let mut hm = HashMap::new();
        for (k, c) in self.category_map.iter() {
            let output_name = c.output.as_ref().unwrap_or(k);
            hm.insert(output_name.clone(), HashSet::new());
        }
        hm
    }

    pub fn icon_for_category(&self, category: &String) -> String {
        if let Some(output) = &self.output
            && let Some(output_category) = output.get(category)
            && let Some(icon) = &output_category.icon
        {
            return icon.clone();
        }
        let icon = format!("applications-{}", category.to_lowercase());
        icon
    }
}

impl ::std::default::Default for Config {
    fn default() -> Self {
        let mut m = HashMap::new();
        m.insert(
            "AudioVideo".into(),
            ConfigCategory::default("Multimedia".into()),
        );
        m.insert("Audio".into(), ConfigCategory::default("Multimedia".into()));
        m.insert("Video".into(), ConfigCategory::default("Multimedia".into()));
        m.insert(
            "Development".into(),
            ConfigCategory::default("Development".into()),
        );
        m.insert(
            "Education".into(),
            ConfigCategory::default("Education".into()),
        );
        m.insert("Game".into(), ConfigCategory::default("Games".into()));
        m.insert(
            "Graphics".into(),
            ConfigCategory::default("Graphics".into()),
        );
        m.insert("Network".into(), ConfigCategory::default("Internet".into()));
        m.insert("Office".into(), ConfigCategory::default("Office".into()));
        m.insert("Science".into(), ConfigCategory::default("Science".into()));
        m.insert(
            "Settings".into(),
            ConfigCategory::default("Settings".into()),
        );
        m.insert("System".into(), ConfigCategory::default("System".into()));
        m.insert("Utility".into(), ConfigCategory::default("Utility".into()));
        Self {
            category_map: m,
            output: None,
        }
    }
}

fn main() -> Result<(), confy::ConfyError> {
    let cfg: Config = confy::load("box-menu-rs", "config")?;

    let locales = get_languages_from_env();
    let entries: Vec<DesktopEntry> = desktop_entries(&locales)
        .into_iter()
        .filter(|x| x.categories().is_some())
        .collect();

    let mut menu_entries = cfg.empty_hash();
    for entry in entries {
        for c in entry
            .categories()
            .unwrap()
            .into_iter()
            .filter(|&k| cfg.category_map.contains_key(k))
        {
            let mapped_category = cfg.category_map.get(c).unwrap();
            let output_name = mapped_category.output.as_ref().map_or(c, |v| &v);
            if let Some(v) = menu_entries.get_mut(output_name) {
                v.insert(Entry {
                    label: escape::escape(entry.full_name(&locales).unwrap_or_default())
                        .to_string(),
                    exec: entry.exec().unwrap_or_default().to_string(),
                    icon: entry.icon().map_or(None, |ei| lookup_icon(ei)),
                });
            }
        }
    }

    println!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    println!(
        "<openbox_menu xmlns=\"http://openbox.org/\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"http://openbox.org/\" >"
    );
    for (category, entries_in_cat) in menu_entries.iter().sorted_by_key(|x| x.0) {
        let category_icon_name = cfg.icon_for_category(category);
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
    Ok(())
}
