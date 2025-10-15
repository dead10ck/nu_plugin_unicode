use std::{
    fmt::Display,
    io::{BufRead, BufReader, Cursor, Read},
};

use encoding_rs_io::DecodeReaderBytesBuilder;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_plugin_unicode_ucd::codegen::{name_aliases::NAME_ALIASES, unicode_data::UNICODE_DATA};
use nu_protocol::{
    IntoValue, LabeledError, ListStream, PipelineData, Range, ShellError, Signals, Signature, Span,
    SyntaxShape, Type, Value,
    ast::PathMember,
    casing::Casing,
    shell_error::io::{self, IoError},
};
use tracing_subscriber::prelude::*;

use crate::{
    Unicode,
    unicode::{
        commands::chars::config::Config,
        constants::{self, commands::chars::flags},
    },
};

pub mod config;

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
        let config = Config::try_from(call)?;

        match input {
            PipelineData::Value(val, meta) => Ok(PipelineData::Value(
                Self::chars(val, &config, engine.signals())?,
                meta,
            )),
            PipelineData::ListStream(stream, meta) => {
                let span = stream.span();
                let stream_signals = signals.clone();

                Ok(PipelineData::ListStream(
                    stream.map({
                        move |val| {
                            Self::chars(val, &config, &stream_signals)
                                .unwrap_or_else(|err| Value::error(ShellError::from(err), span))
                        }
                    }),
                    meta,
                ))
            }
            PipelineData::ByteStream(stream, meta) => {
                let span = stream.span();
                let stream_signals = signals.clone();
                let reader = match stream.reader() {
                    None => return Ok(PipelineData::empty()),
                    Some(r) => r,
                };

                let out_stream = decode_bytes(reader, &config, span, stream_signals)?;

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

    pub(crate) fn chars(
        val: Value,
        config: &Config,
        signals: &Signals,
    ) -> Result<Value, LabeledError> {
        let result = match val {
            str_val @ Value::String { .. } => {
                let span = str_val.span();
                let val = str_val.into_string().unwrap();

                val.chars()
                    .map(|ch| get_unicode_values(ch, span))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_value(Span::unknown())
            }
            Value::List { vals, .. } => vals
                .into_iter()
                .map(|val| Self::chars(val, config, signals))
                .collect::<Result<Vec<Value>, _>>()?
                .into_value(Span::unknown()),
            int_val @ Value::Int { val, .. } => {
                let span = int_val.span();
                get_unicode_values(val, span)?
            }
            ref range_val @ Value::Range { .. } => {
                let span = range_val.span();
                let val = range_val.as_range().unwrap();

                match val {
                    Range::IntRange(range) => range
                        .into_range_iter(signals.clone())
                        .map(|i| Self::chars(i.into_value(span), config, signals))
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
                let bytes = binary_val.as_binary().unwrap();
                let cursor = Cursor::new(bytes);
                Value::list(
                    decode_bytes(cursor, config, span, signals.clone())?.collect(),
                    Span::unknown(),
                )
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

fn get_unicode_values(
    ch: impl TryInto<u32, Error = impl Display>,
    span: Span,
) -> Result<Value, LabeledError> {
    let ch = ch
        .try_into()
        .map_err(|err| LabeledError::new("invalid char").with_label(err.to_string(), span))?;

    let mut data = UNICODE_DATA
        .get(&ch)
        .copied()
        .cloned()
        .map(|data| data.into_value(Span::unknown()))
        .unwrap_or(Value::nothing(Span::unknown()));

    if data.is_nothing() {
        return Ok(data);
    }

    let aliases = NAME_ALIASES.get(&ch).copied().map_or_else(
        || Result::<_, LabeledError>::Ok(Value::nothing(Span::unknown())),
        |aliases| {
            let mut val = aliases.to_vec().into_value(Span::unknown());

            val.remove_data_at_cell_path(&[PathMember::string(
                "codepoint".into(),
                false,
                Casing::Sensitive,
                Span::unknown(),
            )])?;

            Ok(val)
        },
    )?;

    if let Type::Record(_) = data.get_type() {
        data.insert_data_at_cell_path(
            &[PathMember::string(
                "aliases".into(),
                false,
                Casing::Sensitive,
                Span::unknown(),
            )],
            aliases,
            Span::unknown(),
        )?;
    } else {
        return Err(LabeledError::new("unexpected data")
            .with_label(
                "Unicode data returned unexpected result. This is an internal error.",
                data.span(),
            )
            .with_help(format!("{:?}", data)));
    }

    Ok(data)
}

fn decode_bytes<'reader, 'cfg, R: Read + 'reader>(
    reader: R,
    config: &'cfg Config,
    span: Span,
    stream_signals: Signals,
) -> Result<impl Iterator<Item = Value> + use<R>, LabeledError> {
    let mut decoder = BufReader::new(
        DecodeReaderBytesBuilder::new()
            .encoding(Some(config.encoding))
            .bom_override(!config.ignore_bom)
            .build(reader),
    );

    let vals = std::iter::from_fn({
        let config = config.clone();

        move || {
            let buf = match decoder.fill_buf() {
                Ok(bytes) => bytes,
                Err(err) => {
                    return Some(Value::error(
                        ShellError::from(IoError::new(io::ErrorKind::from(err), span, None)),
                        span,
                    ));
                }
            };

            tracing::debug!(phase = "read", input = buf);

            let read_bytes = buf.len();

            if read_bytes == 0 {
                return None;
            }

            let val = UnicodeChars::chars(
                Value::string(unsafe { String::from_utf8_unchecked(buf.to_vec()) }, span),
                &config,
                &stream_signals,
            )
            .unwrap_or_else(|err| Value::error(ShellError::from(err), span));

            decoder.consume(read_bytes);

            Some(val)
        }
    })
    .flat_map(|val| match val {
        Value::List { vals, .. } => vals,
        _ => vec![val],
    });

    Ok(vals)
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
