use freedesktop_desktop_entry::{DesktopEntry, desktop_entries, get_languages_from_env};
use clap::Parser;
use freedesktop_icons::lookup;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::process::Command;
use std::sync::OnceLock;
use std::{collections::{BTreeMap, BTreeSet, HashMap}, path::PathBuf};

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

#[derive(Default)]
struct MenuNode {
    label: String,
    children: BTreeMap<String, MenuNode>,
    entries: BTreeSet<Entry>,
}

impl MenuNode {
    fn new(label: String) -> Self {
        Self {
            label,
            children: BTreeMap::new(),
            entries: BTreeSet::new(),
        }
    }

    fn node_for_path(&mut self, path: &str) -> &mut MenuNode {
        let mut current = self;
        for segment in path.split('/').filter(|segment| !segment.is_empty()) {
            current = current.children.entry(segment.to_string()).or_insert_with(|| {
                MenuNode::new(segment.to_string())
            });
        }
        current
    }

    fn insert(&mut self, path: &str, entry: Entry) {
        self.node_for_path(path).entries.insert(entry);
    }

    fn print(&self, config: &Config, path: &str) {
        if !self.label.is_empty() {
            let category_icon_name = config.icon_for_category(path);
            let icon_str = lookup_icon(&category_icon_name)
                .map(|p| format!(" icon=\"{}\"", p.display()))
                .unwrap_or_default();
            println!(
                "<menu id=\"boxmenu-{}\" label=\"{}\"{}>",
                Self::menu_id(path),
                escape::escape(&self.label),
                icon_str
            );
        }

        for (child_name, child) in &self.children {
            let child_path = if path.is_empty() {
                child_name.clone()
            } else {
                format!("{}/{}", path, child_name)
            };
            child.print(config, &child_path);
        }

        for entry in &self.entries {
            println!("{}", entry);
        }

        if !self.label.is_empty() {
            println!("</menu>");
        }
    }

    fn menu_id(path: &str) -> String {
        path.replace('/', "-").replace(' ', "-")
    }
}

fn theme() -> &'static String {
    static VALUE: OnceLock<String> = OnceLock::new();
    VALUE.get_or_init(|| {
        if let Ok(output) = Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "icon-theme"])
            .output()
        {
            if output.status.success() {
                let theme = String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .trim_matches('"')
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string();
                if !theme.is_empty() {
                    return theme;
                }
            }
            "hicolor".into()
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

#[derive(Debug, Parser)]
#[command(author, version, about = "Generate an Openbox-compatible application menu", long_about = None)]
struct DebugOptions {
    #[arg(long = "debug-program", help = "Inspect icon lookup for a given desktop entry Name")]
    program_name: Option<String>,
}

impl DebugOptions {
    fn program_name(&self) -> Option<&str> {
        self.program_name.as_deref()
    }
}

fn debug_program_icon_resolution(
    entries: &[DesktopEntry],
    locales: &[String],
    config: &Config,
    debug_options: &DebugOptions,
) {
    let name = match debug_options.program_name() {
        Some(name) => name,
        None => return,
    };

    eprintln!("DEBUG: inspecting program '{}'", name);
    eprintln!("DEBUG: icon theme = {}", theme());

    let normalized_name = name.to_lowercase();
    let matching: Vec<_> = entries
        .iter()
        .filter(|entry| entry.full_name(locales).unwrap_or_default().to_lowercase() == normalized_name)
        .collect();

    if matching.is_empty() {
        eprintln!("DEBUG: no desktop entry matched the Name '{}'.", name);
        return;
    }

    for (index, entry) in matching.into_iter().enumerate() {
        let label = entry.full_name(locales).unwrap_or_default();
        let exec = entry.exec().unwrap_or_default();
        let icon_field = entry.icon().unwrap_or_default();
        let entry_icon_path = entry.icon().and_then(lookup_icon);

        eprintln!("DEBUG: match #{}", index + 1);
        eprintln!("  Name: {}", label);
        eprintln!("  Exec: {}", exec);
        eprintln!("  Desktop icon field: {}", if icon_field.is_empty() { "<none>" } else { &icon_field });

        match entry_icon_path {
            Some(path) => eprintln!("  Resolved entry icon: {}", path.display()),
            None if icon_field.is_empty() => {
                eprintln!("  Entry icon is not defined in the desktop file.")
            }
            None => eprintln!("  Entry icon lookup failed for '{}'.", icon_field),
        }

        let categories = entry.categories().unwrap_or_default();
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
                        None => eprintln!("    Category icon lookup failed for '{}'.", category_icon_name),
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
    pub fn empty_tree(&self) -> MenuNode {
        let mut root = MenuNode::new(String::new());
        for (category, config_category) in self.category_map.iter() {
            let output_name = config_category.output.as_deref().unwrap_or(category);
            root.node_for_path(output_name);
        }
        root
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

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let debug_options = DebugOptions::parse();
    let cfg: Config = confy::load("box-menu-rs", "config")?;

    let locales = get_languages_from_env();
    let entries: Vec<DesktopEntry> = desktop_entries(&locales)
        .into_iter()
        .filter(|x| x.categories().is_some())
        .collect();

    if debug_options.program_name().is_some() {
        debug_program_icon_resolution(&entries, &locales, &cfg, &debug_options);
        return Ok(());
    }

    let mut root = cfg.empty_tree();
    for entry in entries {
        for c in entry
            .categories()
            .unwrap()
            .into_iter()
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
    Ok(())
}
