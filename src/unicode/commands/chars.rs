use std::io::{BufRead, BufReader};

use encoding_rs::Encoding;
use encoding_rs_io::DecodeReaderBytesBuilder;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_plugin_unicode_ucd::UNICODE_DATA;
use nu_protocol::{
    IntoValue, LabeledError, ListStream, PipelineData, Range, ShellError, Signals, Signature, Span,
    SyntaxShape, Type, Value,
    shell_error::io::{self, IoError},
};
use tracing_subscriber::prelude::*;

use crate::{
    Unicode,
    unicode::constants::{self, commands::chars::flags},
};

#[derive(Debug)]
pub struct UnicodeChars;

impl UnicodeChars {
    pub(crate) fn run_impl(
        &self,
        _plugin: &Unicode,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let _ = tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
            .with(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
        let signals = engine.signals().clone();

        match input {
            PipelineData::Value(val, meta) => Ok(PipelineData::Value(
                Self::chars(val, engine.signals())?,
                meta,
            )),
            PipelineData::ListStream(stream, meta) => {
                let span = stream.span();

                Ok(PipelineData::ListStream(
                    stream.map(move |val| {
                        Self::chars(val, &signals)
                            .unwrap_or_else(|err| Value::error(ShellError::from(err), span))
                    }),
                    meta,
                ))
            }
            PipelineData::ByteStream(stream, meta) => {
                let span = stream.span();
                let stream_signals = signals.clone();
                let encoding = call
                    .get_flag_value(constants::commands::chars::flags::ENCODING)
                    .map(|val| val.into_string().unwrap())
                    .unwrap_or(constants::commands::chars::defaults::ENCODING.into());

                let ignore_bom = call.has_flag(constants::commands::chars::flags::IGNORE_BOM)?;

                let mut reader = match stream.reader() {
                    None => return Ok(PipelineData::empty()),
                    Some(r) => {
                        let decoder = DecodeReaderBytesBuilder::new()
                            .encoding(Encoding::for_label_no_replacement(encoding.as_bytes()))
                            .bom_override(!ignore_bom)
                            .build(r);

                        BufReader::new(decoder)
                    }
                };

                let out_stream = std::iter::from_fn(move || {
                    let buf = match reader.fill_buf() {
                        Ok(bytes) => bytes,
                        Err(err) => {
                            return Some(Value::error(
                                ShellError::from(IoError::new(
                                    io::ErrorKind::from(err),
                                    span,
                                    None,
                                )),
                                span,
                            ));
                        }
                    };

                    tracing::debug!(phase = "read", input = buf);

                    let read_bytes = buf.len();

                    if read_bytes == 0 {
                        return None;
                    }

                    let val = Self::chars(
                        Value::string(unsafe { String::from_utf8_unchecked(buf.to_vec()) }, span),
                        &stream_signals,
                    )
                    .unwrap_or_else(|err| Value::error(ShellError::from(err), span));

                    reader.consume(read_bytes);

                    Some(val)
                })
                .flat_map(|val| match val {
                    Value::List { vals, .. } => vals,
                    _ => vec![val],
                });

                Ok(PipelineData::ListStream(
                    ListStream::new(out_stream, span, signals),
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
            binary_val @ Value::Binary { .. } => {
                let span = binary_val.span();
                let val = binary_val.as_binary().unwrap();
                let str = String::from_utf8(val.into()).map_err(|err| {
                    LabeledError::new("non-utf-8 bytes").with_label(err.to_string(), span)
                })?;
                Self::chars(str.into_value(span), signals)?
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
        constants::commands::chars::NAME
    }

    fn description(&self) -> &str {
        "Splits the input string into code points and returns metadata about each"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build(self.name()).input_output_types(vec![
            (Type::String, Type::Table([].into())),
            (Type::Binary, Type::Table([].into())),
            (Type::Int, Type::Table([].into())),
            (Type::Range, Type::Table([].into())), (Type::List(Box::new(Type::Any)), Type::Table([].into())),
        ])
        .named(flags::ENCODING, SyntaxShape::String, "Encoding of the input bytes. By default, BOM sniffing occurs to detect the encoding; failing that, UTF-8 is assumed.", Some('e'))
        .switch(flags::IGNORE_BOM, "Ignore the BOM, if present. By default, even if an encoding is specified, if a BOM is present, the encoding from the command line is ignored.", Some('b'))
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
