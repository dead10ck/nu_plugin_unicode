use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{IntoValue, LabeledError, PipelineData, Signature, Span, Type, Value};
use tracing_subscriber::prelude::*;

use crate::{Unicode, ucd, unicode::constants};

#[derive(Debug)]
pub struct UnicodeChars;

impl UnicodeChars {
    pub(crate) fn run_impl(
        &self,
        _plugin: &Unicode,
        _engine: &EngineInterface,
        _call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let _ = tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
            .with(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();

        match input {
            PipelineData::Value(val, _) => {
                let result = match val {
                    Value::String { val, .. } => val
                        .chars()
                        .map(|ch| {
                            ucd::names::NAMES
                                .get(&ch)
                                .cloned()
                                .into_value(Span::unknown())
                        })
                        .collect::<Vec<_>>()
                        .into_value(Span::unknown()),
                    val => {
                        return Err(LabeledError::new("Invalid input")
                            .with_label("Should be string or utf-8 bytes", val.span()));
                    }
                };
                let result = PipelineData::Value(result, None);
                tracing::trace!(phase = "return", ?result);
                Ok(result)
            }
            PipelineData::ListStream(_stream, _) => {
                todo!();
                // let span = stream.span();

                // Ok(PipelineData::ListStream(
                //     ListStream::new(std::iter::empty(), span, engine.signals().clone()),
                //     None,
                // ))
            }
            data => Err(LabeledError::new("invalid input").with_label(
                "Only values can be passed as input",
                data.span().unwrap_or(Span::unknown()),
            )),
        }
    }
}

impl PluginCommand for UnicodeChars {
    type Plugin = Unicode;

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        self.run_impl(plugin, engine, call, input)
    }

    fn name(&self) -> &str {
        constants::commands::CHARS
    }

    fn description(&self) -> &str {
        "Splits the input string into code points and returns metadata about each"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::String, Type::Table([].into()))])
    }

    fn examples(&self) -> Vec<nu_protocol::Example<'static>> {
        vec![
            // Example {
            //     example: "dns query google.com",
            //     description: "simple query for A / AAAA records",
            //     result: None,
            // },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["unicode", "string"]
    }
}
