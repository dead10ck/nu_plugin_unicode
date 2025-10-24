use std::path::PathBuf;

use encoding_rs::Encoding;
use nu_plugin::EvaluatedCall;
use nu_protocol::{LabeledError, Span, Value};

use crate::unicode::{
    commands::ucd::index::get_index_dir,
    constants::commands::chars::{defaults, flags},
};

#[derive(Clone)]
pub struct Config {
    pub encoding: &'static Encoding,
    pub ignore_bom: bool,
    pub index_dir: PathBuf,
}

impl TryFrom<&EvaluatedCall> for Config {
    type Error = LabeledError;

    fn try_from(call: &EvaluatedCall) -> Result<Self, Self::Error> {
        let ignore_bom = call.has_flag(flags::IGNORE_BOM.name)?;

        let encoding_name = call
            .get_flag_value(flags::ENCODING.name)
            .unwrap_or(Value::string(defaults::ENCODING, Span::unknown()));
        let encoding_name_span = encoding_name.span();

        let encoding =
            Encoding::for_label_no_replacement(encoding_name.into_string().unwrap().as_bytes())
                .ok_or_else(|| {
                    LabeledError::new("encoding not found")
                        .with_label("no such encoding", encoding_name_span)
                })?;

        let index_dir = get_index_dir(call)?;

        Ok(Config {
            encoding,
            ignore_bom,
            index_dir,
        })
    }
}
