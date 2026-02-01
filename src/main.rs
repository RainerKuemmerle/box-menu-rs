use freedesktop_desktop_entry::{DesktopEntry, desktop_entries, get_languages_from_env};
use freedesktop_icons::{default_theme_gtk, lookup};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::OnceLock;
use std::{collections::HashMap, collections::HashSet, path::PathBuf};

const CATEGORY_ICON_PREFIX: &str = "applications-";
const OPENBOX_XMLNS: &str = "http://openbox.org/";
const OPENBOX_XSI: &str = "http://www.w3.org/2001/XMLSchema-instance";

mod escape;

#[derive(Eq, Hash, PartialEq, PartialOrd, Ord)]
struct Entry {
    label: String,
    exec: String,
    icon: Option<PathBuf>,
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let icon_attr = self
            .icon
            .as_ref()
            .map(|p| format!(" icon=\"{}\"", p.display()))
            .unwrap_or_default();
        write!(
            f,
            "<item label=\"{}\"{}><action name=\"Execute\"><command>{}</command></action></item>",
            self.label, icon_attr, self.exec,
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
        self.category_map
            .iter()
            .map(|(k, c)| (c.output.as_ref().unwrap_or(k).clone(), HashSet::new()))
            .collect()
    }

    pub fn icon_for_category(&self, category: &str) -> String {
        self.output
            .as_ref()
            .and_then(|output| output.get(category))
            .and_then(|oc| oc.icon.as_ref())
            .cloned()
            .unwrap_or_else(|| format!("{}{}", CATEGORY_ICON_PREFIX, category.to_lowercase()))
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
            let output_name = mapped_category.output.as_ref().map_or(c, |v| v);
            if let Some(v) = menu_entries.get_mut(output_name) {
                v.insert(Entry {
                    label: escape::escape(entry.full_name(&locales).unwrap_or_default())
                        .to_string(),
                    exec: entry.exec().unwrap_or_default().to_string(),
                    icon: entry.icon().and_then(lookup_icon),
                });
            }
        }
    }

    println!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    println!(
        "<openbox_menu xmlns=\"{}\" xmlns:xsi=\"{}\" xsi:schemaLocation=\"{}\" >",
        OPENBOX_XMLNS, OPENBOX_XSI, OPENBOX_XMLNS
    );
    for (category, entries_in_cat) in menu_entries.iter().sorted_by_key(|x| x.0) {
        let category_icon_name = cfg.icon_for_category(category);
        let icon_str = lookup_icon(&category_icon_name)
            .map(|p| format!(" icon=\"{}\"", p.display()))
            .unwrap_or_default();
        println!("<menu id=\"boxmenu-{category}\" label=\"{category}\" {icon_str}>");
        for e in entries_in_cat.iter().sorted() {
            println!("{e}");
        }
        println!("</menu>");
    }
    println!("</openbox_menu>");
    Ok(())
}
