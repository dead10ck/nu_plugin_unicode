use std::path::PathBuf;

use nu_plugin::EvaluatedCall;
use nu_protocol::{LabeledError, Span};

use crate::unicode::constants::commands::ucd::index;

pub mod build;
pub mod df;

pub fn get_index_dir(call: &EvaluatedCall) -> Result<PathBuf, LabeledError> {
    let index_dir = call
        .get_flag_value(index::common::INDEX_DIR.name)
        .map(|val| Some(PathBuf::from(val.into_string().unwrap())))
        .unwrap_or_else(|| dirs::data_dir().map(|dir| dir.join(index::DATA_DIR_NAME)))
        .ok_or_else(|| {
            LabeledError::new("missing data dir")
                .with_label("system is missing an app data directory", Span::unknown())
        })?;

    Ok(index_dir)
}
