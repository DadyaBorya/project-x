use crate::models::config_file::ConfigArguments;

#[derive(Clone)]
pub struct Payload {
    pub value: String,
    pub arg: ConfigArguments,
}