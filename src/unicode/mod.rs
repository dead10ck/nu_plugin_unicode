use nu_plugin::{Plugin, PluginCommand};

pub mod commands;
pub mod constants;

pub struct Unicode;

impl Plugin for Unicode {
    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![Box::new(commands::chars::UnicodeChars)]
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").into()
    }
}

impl Unicode {
    pub const PLUGIN_NAME: &str = "unicode";
}
