use crate::models::config_file::ConfigFile;
use crate::models::payload::Payload;

#[derive(Default)]
pub struct State {
    pub list: Vec<ConfigFile>,
    pub current_name: Option<String>,
    pub payloads: Vec<Payload>
}