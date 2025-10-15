use std::collections::BTreeMap;
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use ucd_parse::{
    Codepoint, UcdFile, UcdFileByCodepoint, 
};

fn main() {
    let ucd_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("ucd");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    generate_unicode_data(&ucd_dir, &out_dir);
    generate_name_aliases(&ucd_dir, &out_dir);
}

fn generate_ucd_source<U, F>(ucd_dir: &Path, out_path: &Path, generate_entries: F)
where
    U: UcdFile + UcdFileByCodepoint,
    F: Fn(BufWriter<File>, BTreeMap<Codepoint, Vec<U>>, phf_codegen::Map<u32>),
{
    let phf_source_file = BufWriter::new(File::create(out_path).unwrap());
    let parsed = ucd_parse::parse_many_by_codepoint::<_, U>(ucd_dir).unwrap();

    println!(
        "cargo::rerun-if-changed={}",
        U::file_path(ucd_dir).to_str().unwrap()
    );
    println!("cargo::rerun-if-changed={}", out_path.to_str().unwrap());

    let phf_source = phf_codegen::Map::<u32>::new();

    generate_entries(phf_source_file, parsed, phf_source);
}

fn generate_unicode_data(ucd_dir: &Path, out_dir: &Path) {
    generate_ucd_source(
        ucd_dir,
        &out_dir.join("unicode_data.rs"),
        |mut writer, parsed, mut phf_source| {
            for (codepoint, mut data) in parsed.into_iter() {
                assert_eq!(1, data.len());
                let data = data.pop().unwrap();

                phf_source.entry(
                    codepoint.value(),
                    format!("&{:?}", UnicodeDataLiteral(data)),
                );
            }

            writeln!(
                &mut writer,
                "use ucd_parse::{{UnicodeDataDecompositionTag, UnicodeDataNumeric}};\n",
            )
            .unwrap();

            writeln!(
                &mut writer,
                "pub static UNICODE_DATA: phf::Map<u32, &UnicodeDataStatic> = {};\n",
                phf_source.build()
            )
            .unwrap();
        },
    );
}

fn generate_name_aliases(ucd_dir: &Path, out_dir: &Path) {
    generate_ucd_source(
        ucd_dir,
        &out_dir.join("name_aliases.rs"),
        |mut writer, parsed: BTreeMap<Codepoint, Vec<ucd_parse::NameAlias>>, mut phf_source| {
            for (codepoint, aliases) in parsed.into_iter() {
                phf_source.entry(codepoint.value(), format!("&{:?}", aliases.as_slice()));
            }

            writeln!(&mut writer, "use ucd_parse::NameAliasLabel;\n",).unwrap();

            writeln!(
                &mut writer,
                "pub static NAME_ALIASES: phf::Map<u32, &[ucd_parse::NameAlias]> = {};\n",
                phf_source.build()
            )
            .unwrap();
        },
    )
}

mod build_types;
