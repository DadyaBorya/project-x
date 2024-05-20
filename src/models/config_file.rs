use std::path::{PathBuf};
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct ConfigFile {
    pub name: String,
    pub repo_url: String,
    pub desc: String,
    pub files: Vec<OsFile>,
    pub arguments: Vec<ConfigArguments>,
}

#[derive(Deserialize, Clone)]
pub struct OsFile {
    pub os: String,
    pub arch: String,
    pub path: PathBuf,
}

#[derive(Deserialize, Clone)]
pub struct ConfigArguments {
    pub name: String,
    pub r#type: String,
    pub desc: String,
    pub optional: bool,
}