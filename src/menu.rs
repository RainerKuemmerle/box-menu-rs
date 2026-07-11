use crate::{config::Config, escape, icon::lookup_icon};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    path::PathBuf,
};

#[derive(Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct Entry {
    pub label: String,
    pub exec: String,
    pub icon: Option<PathBuf>,
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

pub struct MenuNode {
    label: String,
    children: BTreeMap<String, MenuNode>,
    entries: BTreeSet<Entry>,
}

impl MenuNode {
    pub fn new(label: String) -> Self {
        Self {
            label,
            children: BTreeMap::new(),
            entries: BTreeSet::new(),
        }
    }

    pub fn node_for_path(&mut self, path: &str) -> &mut MenuNode {
        let mut current = self;
        for segment in path.split('/').filter(|segment| !segment.is_empty()) {
            current = current
                .children
                .entry(segment.to_string())
                .or_insert_with(|| MenuNode::new(segment.to_string()));
        }
        current
    }

    pub fn insert(&mut self, path: &str, entry: Entry) {
        self.node_for_path(path).entries.insert(entry);
    }

    pub fn print(&self, config: &Config, path: &str) {
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
        path.replace(['/', ' '], "-")
    }
}
