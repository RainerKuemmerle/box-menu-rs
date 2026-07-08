use crate::menu::MenuNode;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

#[derive(Serialize, Deserialize)]
pub struct ConfigCategory {
    pub output: Option<String>,
}

impl ConfigCategory {
    pub fn default(output: String) -> Self {
        Self {
            output: Some(output),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct OutputCategory {
    pub icon: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Options {
    pub visibility_filter: bool,
    pub icon_theme: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            visibility_filter: true,
            icon_theme: None,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub category_map: HashMap<String, ConfigCategory>,
    pub output: Option<HashMap<String, OutputCategory>>,
    #[serde(default)]
    pub options: Options,
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
            .unwrap_or_else(|| format!("applications-{}", category.to_lowercase()))
    }
}

impl Default for Config {
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
            options: Options::default(),
        }
    }
}

pub fn load_config(config_file: Option<&PathBuf>) -> Result<Config, Box<dyn std::error::Error>> {
    if let Some(path) = config_file {
        let contents = std::fs::read_to_string(path)?;
        let cfg: Config = serde_yaml::from_str(&contents)?;
        Ok(cfg)
    } else {
        let cfg: Config = confy::load("box-menu-rs", "config")?;
        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn load_config_reads_yaml_file() {
        let yaml = r#"
category_map:
  TestCategory:
    output: Testing
output:
  Testing:
    icon: test-icon
options:
  visibility_filter: false
"#;

        let temp_dir = std::env::temp_dir();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let config_path = temp_dir.join(format!("box-menu-rs-test-config-{}.yml", timestamp));

        fs::write(&config_path, yaml).expect("failed to write test config file");
        let cfg = load_config(Some(&config_path)).expect("failed to load config");

        assert_eq!(cfg.options.visibility_filter, false);
        assert_eq!(cfg.category_map["TestCategory"].output.as_deref(), Some("Testing"));

        let output = cfg.output.expect("output section missing");
        assert_eq!(output["Testing"].icon.as_deref(), Some("test-icon"));

        fs::remove_file(&config_path).expect("failed to remove test config file");
    }
}
