use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_plugin_unicode_ucd::UNICODE_DATA;
use nu_protocol::{
    IntoValue, LabeledError, PipelineData, Range, ShellError, Signals, Signature, Span, Type, Value,
};
use tracing_subscriber::prelude::*;

use crate::{Unicode, unicode::constants};

#[derive(Debug)]
pub struct UnicodeChars;

impl UnicodeChars {
    pub(crate) fn run_impl(
        &self,
        _plugin: &Unicode,
        engine: &EngineInterface,
        _call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let _ = tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
            .with(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();

        match input {
            PipelineData::Value(val, meta) => Ok(PipelineData::Value(
                Self::chars(val, engine.signals())?,
                meta,
            )),
            PipelineData::ListStream(stream, meta) => {
                let span = stream.span();
                let signals = engine.signals().clone();

                Ok(PipelineData::ListStream(
                    stream.map(move |val| {
                        Self::chars(val, &signals)
                            .unwrap_or_else(|err| Value::error(ShellError::from(err), span))
                    }),
                    meta,
                ))
            }
            data => Err(LabeledError::new("invalid input").with_label(
                "Only values can be passed as input",
                data.span().unwrap_or(Span::unknown()),
            )),
        }
    }
    pub(crate) fn chars(val: Value, signals: &Signals) -> Result<Value, LabeledError> {
        let result = match val {
            Value::String { val, .. } => val
                .chars()
                .map(|ch| {
                    UNICODE_DATA
                        .get(&(ch as u32))
                        .copied()
                        .cloned()
                        .into_value(Span::unknown())
                })
                .collect::<Vec<_>>()
                .into_value(Span::unknown()),
            Value::List { vals, .. } => vals
                .into_iter()
                .map(|val| Self::chars(val, signals))
                .collect::<Result<Vec<Value>, _>>()?
                .into_value(Span::unknown()),
            int_val @ Value::Int { val, .. } => {
                let span = int_val.span();

                UNICODE_DATA
                    .get(&u32::try_from(val).map_err(|err| {
                        LabeledError::new("invalid char").with_label(err.to_string(), span)
                    })?)
                    .copied()
                    .cloned()
                    .map(|data| data.into_value(Span::unknown()))
                    .unwrap_or(Value::nothing(Span::unknown()))
            }
            ref range_val @ Value::Range { .. } => {
                let span = range_val.span();
                let val = range_val.as_range().unwrap();

                match val {
                    Range::IntRange(range) => range
                        .into_range_iter(signals.clone())
                        .map(|i| Self::chars(i.into_value(span), signals))
                        .collect::<Result<Vec<_>, _>>()?
                        .into_value(span),
                    Range::FloatRange(_) => {
                        return Err(LabeledError::new("Invalid input").with_label(
                            "Input could not be converted to a Unicode codepoint",
                            span,
                        ));
                    }
                }
            }
            val => {
                return Err(LabeledError::new("Invalid input").with_label(
                    "Input could not be converted to a Unicode codepoint",
                    val.span(),
                ));
            }
        };
        tracing::trace!(phase = "return", ?result);
        Ok(result)
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
        Signature::build(self.name()).input_output_types(vec![
            (Type::String, Type::Table([].into())),
            (Type::Range, Type::Table([].into())),
            (Type::List(Box::new(Type::Any)), Type::Table([].into())),
        ])
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
