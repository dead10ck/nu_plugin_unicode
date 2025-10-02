use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use ucd_parse::{UnicodeData, UnicodeDataDecomposition};

fn main() {
    let ucd_dir = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("ucd");
    let phf_source_path = Path::new(&env::var("OUT_DIR").unwrap()).join("codegen.rs");
    let mut phf_source_file = BufWriter::new(File::create(&phf_source_path).unwrap());

    let unicode_data = ucd_parse::parse_many_by_codepoint::<_, UnicodeData>(ucd_dir).unwrap();

    let mut phf_source = phf_codegen::Map::<u32>::new();

    for (codepoint, data) in unicode_data.into_iter().take(10) {
        let data = data.into_iter().map(UnicodeDataLiteral).collect::<Vec<_>>();
        phf_source.entry(codepoint.value(), format!("&{:?}", data));
    }

    writeln!(
        &mut phf_source_file,
        "pub static UNICODE_DATA: phf::Map<u32, &[UnicodeDataStatic]> = {};\n",
        phf_source.build()
    )
    .unwrap();
}

struct UnicodeDataDecompositionLiteral<'a>(&'a UnicodeDataDecomposition);

impl std::fmt::Debug for UnicodeDataDecompositionLiteral<'_> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("UnicodeDataDecompositionStatic")
            .field("tag", &self.0.tag)
            .field("len", &self.0.len)
            .field(
                "mapping",
                &self
                    .0
                    .mapping
                    .iter()
                    .map(|cp| cp.value())
                    .collect::<Vec<_>>()
                    .as_slice(),
            )
            .finish()
    }
}

struct UnicodeDataLiteral(UnicodeData);

impl std::fmt::Debug for UnicodeDataLiteral {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("UnicodeDataStatic")
            .field("codepoint", &self.0.codepoint.value())
            .field("name", &self.0.name)
            .field("general_category", &self.0.general_category)
            .field(
                "canonical_combining_class",
                &self.0.canonical_combining_class,
            )
            .field("bidi_class", &self.0.bidi_class)
            .field(
                "decomposition",
                &UnicodeDataDecompositionLiteral(&self.0.decomposition),
            )
            .field("numeric_type_decimal", &self.0.numeric_type_decimal)
            .field("numeric_type_digit", &self.0.numeric_type_digit)
            .field("numeric_type_numeric", &self.0.numeric_type_numeric)
            .field("bidi_mirrored", &self.0.bidi_mirrored)
            .field("unicode1_name", &self.0.unicode1_name)
            .field("iso_comment", &self.0.iso_comment)
            .field("simple_uppercase_mapping", &self.0.simple_uppercase_mapping)
            .field("simple_lowercase_mapping", &self.0.simple_lowercase_mapping)
            .field("simple_titlecase_mapping", &self.0.simple_titlecase_mapping)
            .finish()
    }
}
