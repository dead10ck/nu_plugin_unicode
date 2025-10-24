use std::{path::PathBuf, sync::Arc};

use datafusion::{
    arrow::{
        array::{self, RecordBatch, UInt32Builder},
        datatypes::{DataType, Field, Fields, Schema},
    },
    dataframe::DataFrameWriteOptions,
    prelude::SessionContext,
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    IntoSpanned, LabeledError, PipelineData, Signals, Signature, Span, Spanned, Type,
};
use tracing_subscriber::prelude::*;
use ucd_parse::{Codepoint, UnicodeData, UnicodeDataDecomposition};

use crate::{
    Unicode,
    unicode::{
        commands::ucd::index::get_index_dir,
        constants::{
            self,
            commands::ucd::index::{
                build::flags,
                dataframe::{persist::path, table},
            },
        },
    },
};

#[derive(Debug)]
pub struct UcdIndexBuild;

impl UcdIndexBuild {
    pub(crate) async fn run_impl(
        &self,
        _plugin: &Unicode,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let _ = tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
            .with(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();

        let ucd_dir = call
            .get_flag_value(flags::UCD_DIR.name)
            .ok_or_else(|| {
                LabeledError::new("missing UCD directory")
                    .with_label(format!("--{} is required", flags::UCD_DIR.name), call.head)
            })
            .map(|val| {
                let span = val.span();
                PathBuf::from(val.into_string().unwrap()).into_spanned(span)
            })?;

        let index_dir = get_index_dir(call)?;
        let signals = engine.signals().clone();

        build_index(ucd_dir, index_dir, signals).await?;

        Ok(PipelineData::Empty)
    }
}

async fn build_index(
    ucd_dir: Spanned<PathBuf>,
    index_dir: PathBuf,
    _signals: Signals,
) -> Result<(), LabeledError> {
    let decomposition_map_codepoint_field =
        Arc::new(Field::new("codepoints", DataType::UInt32, false));
    let decomposition_type = Fields::from_iter([
        Field::new("tag", DataType::Utf8, true),
        Field::new(
            "mapping",
            DataType::List(decomposition_map_codepoint_field.clone()),
            true,
        ),
    ]);

    let numeric_type_numeric_fields = Fields::from_iter([
        Field::new("numerator", DataType::Int64, false),
        Field::new("denominator", DataType::Int64, true),
    ]);

    let unicode_data_schema = Schema::new(vec![
        Field::new("codepoint", DataType::UInt32, false),
        Field::new("name", DataType::Utf8, true),
        Field::new("general_category", DataType::Utf8, true),
        Field::new("canonical_combining_class", DataType::UInt8, true),
        Field::new("bidi_class", DataType::Utf8, true),
        Field::new(
            "decomposition",
            DataType::Struct(decomposition_type.clone()),
            true,
        ),
        Field::new("numeric_type_decimal", DataType::UInt8, true),
        Field::new("numeric_type_digit", DataType::UInt8, true),
        Field::new(
            "numeric_type_numeric",
            DataType::Struct(numeric_type_numeric_fields.clone()),
            true,
        ),
        Field::new("bidi_mirrored", DataType::Boolean, false),
        Field::new("unicode1_name", DataType::Utf8, true),
        Field::new("iso_comment", DataType::Utf8, true),
        Field::new("simple_uppercase_mapping", DataType::UInt32, true),
        Field::new("simple_lowercase_mapping", DataType::UInt32, true),
        Field::new("simple_titlecase_mapping", DataType::UInt32, true),
    ]);

    let unicode_data = ucd_parse::parse::<_, UnicodeData>(&ucd_dir.item).map_err(|err| {
        LabeledError::new("failed to parse UCD data").with_label(err.to_string(), ucd_dir.span)
    })?;

    let ctx = SessionContext::new();

    let mut codepoint = array::UInt32Builder::with_capacity(unicode_data.len());
    let mut name = array::StringBuilder::with_capacity(
        unicode_data.len(),
        unicode_data.iter().map(|data| data.name.len()).sum(),
    );
    let mut general_category = array::StringBuilder::with_capacity(
        unicode_data.len(),
        unicode_data
            .iter()
            .map(|data| data.general_category.len())
            .sum(),
    );
    let mut canonical_combining_class = array::UInt8Builder::with_capacity(unicode_data.len());
    let mut bidi_class = array::StringBuilder::with_capacity(
        unicode_data.len(),
        unicode_data.iter().map(|data| data.bidi_class.len()).sum(),
    );

    let decomposition_tag = array::StringBuilder::new();
    let decomposition_mapping = array::ListBuilder::new(array::UInt32Builder::new())
        .with_field(decomposition_map_codepoint_field);
    let mut decomposition = array::StructBuilder::new(
        decomposition_type,
        vec![Box::new(decomposition_tag), Box::new(decomposition_mapping)],
    );

    let mut numeric_type_decimal = array::UInt8Builder::with_capacity(unicode_data.len());
    let mut numeric_type_digit = array::UInt8Builder::with_capacity(unicode_data.len());

    // UnionBuilder does not support complex types like fixed size lists so we
    // have to build it up ourselves
    let mut numeric_type_numeric = array::StructBuilder::new(
        numeric_type_numeric_fields,
        vec![
            Box::new(array::Int64Builder::with_capacity(unicode_data.len())),
            Box::new(array::Int64Builder::with_capacity(unicode_data.len())),
        ],
    );

    let mut bidi_mirrored = array::BooleanBuilder::with_capacity(unicode_data.len());
    let mut unicode1_name = array::StringBuilder::with_capacity(
        unicode_data.len(),
        unicode_data
            .iter()
            .map(|data| data.unicode1_name.len())
            .sum(),
    );
    let mut iso_comment = array::StringBuilder::with_capacity(
        unicode_data.len(),
        unicode_data.iter().map(|data| data.iso_comment.len()).sum(),
    );
    let mut simple_uppercase_mapping = array::UInt32Builder::with_capacity(unicode_data.len());
    let mut simple_lowercase_mapping = array::UInt32Builder::with_capacity(unicode_data.len());
    let mut simple_titlecase_mapping = array::UInt32Builder::with_capacity(unicode_data.len());

    for entry in unicode_data.into_iter() {
        let cp = entry.codepoint.value();

        codepoint.append_value(cp);
        name.append_value(entry.name);
        general_category.append_value(entry.general_category);
        canonical_combining_class.append_value(entry.canonical_combining_class);
        bidi_class.append_value(entry.bidi_class);
        build_decomposition(cp, &mut decomposition, entry.decomposition);
        numeric_type_decimal.append_option(entry.numeric_type_decimal);
        numeric_type_digit.append_option(entry.numeric_type_digit);

        match entry.numeric_type_numeric {
            None => {
                numeric_type_numeric
                    .field_builder::<array::Int64Builder>(0)
                    .expect("numeric_type_numeric numerator builder should be Int64")
                    .append_null();
                numeric_type_numeric
                    .field_builder::<array::Int64Builder>(1)
                    .expect("numeric_type_numeric denominator builder should be Int64")
                    .append_null();
                numeric_type_numeric.append_null();
            }
            Some(numeric) => match numeric {
                ucd_parse::UnicodeDataNumeric::Integer(i) => {
                    numeric_type_numeric
                        .field_builder::<array::Int64Builder>(0)
                        .expect("numeric_type_numeric numerator builder should be Int64")
                        .append_value(i);
                    numeric_type_numeric
                        .field_builder::<array::Int64Builder>(1)
                        .expect("numeric_type_numeric denominator builder should be Int64")
                        .append_null();
                    numeric_type_numeric.append(true);
                }
                ucd_parse::UnicodeDataNumeric::Rational(num, denom) => {
                    numeric_type_numeric
                        .field_builder::<array::Int64Builder>(0)
                        .expect("numeric_type_numeric numerator builder should be Int64")
                        .append_value(num);
                    numeric_type_numeric
                        .field_builder::<array::Int64Builder>(1)
                        .expect("numeric_type_numeric denominator builder should be Int64")
                        .append_value(denom);
                    numeric_type_numeric.append(true);
                }
            },
        }

        bidi_mirrored.append_value(entry.bidi_mirrored);
        unicode1_name.append_value(entry.unicode1_name);
        iso_comment.append_value(entry.iso_comment);
        simple_uppercase_mapping
            .append_option(entry.simple_uppercase_mapping.map(Codepoint::value));
        simple_lowercase_mapping
            .append_option(entry.simple_lowercase_mapping.map(Codepoint::value));
        simple_titlecase_mapping
            .append_option(entry.simple_titlecase_mapping.map(Codepoint::value));
    }

    let batch = RecordBatch::try_new(
        Arc::new(unicode_data_schema),
        vec![
            Arc::new(codepoint.finish()),
            Arc::new(name.finish()),
            Arc::new(general_category.finish()),
            Arc::new(canonical_combining_class.finish()),
            Arc::new(bidi_class.finish()),
            Arc::new(decomposition.finish()),
            Arc::new(numeric_type_decimal.finish()),
            Arc::new(numeric_type_digit.finish()),
            Arc::new(numeric_type_numeric.finish()),
            Arc::new(bidi_mirrored.finish()),
            Arc::new(unicode1_name.finish()),
            Arc::new(iso_comment.finish()),
            Arc::new(simple_uppercase_mapping.finish()),
            Arc::new(simple_lowercase_mapping.finish()),
            Arc::new(simple_titlecase_mapping.finish()),
        ],
    )
    .map_err(|err| {
        LabeledError::new("error creating table batch").with_label(err.to_string(), ucd_dir.span)
    })?;

    ctx.register_batch(table::name::UNICODE_DATA, batch)
        .map_err(|err| {
            LabeledError::new("error registering table batch with session context")
                .with_label(err.to_string(), ucd_dir.span)
        })?;

    let df = ctx.table(table::name::UNICODE_DATA).await.map_err(|err| {
        LabeledError::new("error creating dataframe").with_label(err.to_string(), Span::unknown())
    })?;

    let out_path = index_dir.join(path::UNICODE_DATA);

    df.write_parquet(
        out_path.to_str().expect("non-utf8 index path"),
        DataFrameWriteOptions::new(),
        // UNSUPPORTED??
        //     .with_insert_operation(datafusion::logical_expr::dml::InsertOp::Overwrite),
        None,
    )
    .await
    .map_err(|err| {
        LabeledError::new("error writing index").with_label(err.to_string(), Span::unknown())
    })?;

    Ok(())
}

fn build_decomposition(
    cp: u32,
    decomposition_builder: &mut array::StructBuilder,
    decomposition: UnicodeDataDecomposition,
) {
    let mut valid = decomposition.tag.is_some();

    let tag = decomposition_builder
        .field_builder::<array::StringBuilder>(0)
        .expect("decomposition tag builder should be string");

    tag.append_option(decomposition.tag.map(|tag| tag.to_string()));

    let mapping = decomposition_builder
        .field_builder::<array::ListBuilder<UInt32Builder>>(1)
        .expect("decomposition mapping builder should be list of uint32");

    // ucd-parse will by default make this array of mappings when none exist
    // in the UCD. Just represent this with null instead.
    let default_mapping = [cp, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    if decomposition.mapping != default_mapping {
        valid = true;

        let vals = decomposition
            .mapping
            .into_iter()
            .map(Codepoint::value)
            .filter(|cp| *cp != 0)
            .collect::<Vec<_>>();

        mapping
            .values()
            .append_values(&vals, &vals.iter().map(|_| true).collect::<Vec<_>>());
    }

    mapping.append(valid);
    decomposition_builder.append(valid);
}

impl PluginCommand for UcdIndexBuild {
    type Plugin = Unicode;

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let rt = tokio::runtime::Runtime::new().map_err(|err| {
            LabeledError::new("error creating tokio runtime")
                .with_label(err.to_string(), Span::unknown())
        })?;

        rt.block_on(self.run_impl(plugin, engine, call, input))
    }

    fn name(&self) -> &str {
        constants::commands::ucd::index::build::NAME
    }

    fn description(&self) -> &str {
        "Indexes the plain text UCD data files for searching"
    }

    fn signature(&self) -> nu_protocol::Signature {
        let mut sig = Signature::build(self.name())
            .input_output_types(vec![(Type::Nothing, Type::Table([].into()))]);

        for flag in flags::FLAGS {
            if let Some(ref shape) = flag.shape {
                sig = sig.named(flag.name, shape.clone(), flag.desc, flag.short)
            } else {
                sig = sig.switch(flag.name, flag.desc, flag.short)
            }
        }

        sig
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
