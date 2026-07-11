use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(author, version, about = "Generate an Openbox-compatible application menu", long_about = None)]
pub struct CliOptions {
    #[arg(
        value_name = "NAME",
        help = "Desktop entry Name substring to inspect when using --list program.",
        required_if_eq("list", "program")
    )]
    program_name: Option<String>,

    #[arg(
        long = "config-file",
        value_name = "PATH",
        help = "Load configuration from a specific YAML file instead of the default XDG config"
    )]
    config_file: Option<PathBuf>,

    #[arg(
        long = "list",
        value_name = "ACTION",
        help = "List discovered desktop entries by action."
    )]
    list: Option<ListAction>,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum ListAction {
    #[value(help = "All discovered desktop entries.")]
    All,
    #[value(help = "Entries with missing entry icon lookup.")]
    MissingIcons,
    #[value(help = "Hidden entries excluded by visibility filtering.")]
    Excluded,
    #[value(help = "Inspect a specific entry by Name.")]
    Program,
}

impl CliOptions {
    pub fn program_name(&self) -> Option<&str> {
        self.program_name.as_deref()
    }

    pub fn config_file(&self) -> Option<&PathBuf> {
        self.config_file.as_ref()
    }

    pub fn list_action(&self) -> Option<ListAction> {
        self.list
    }
}
