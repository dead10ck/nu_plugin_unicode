use std::{collections::HashMap, sync::LazyLock};

use nu_protocol::IntoValue;

pub mod generated;

const BIT_CODEPOINT_MASK: u64 = (1 << 33) - 1;
const BIT_UNICODE_DATA_TXT: u64 = 1 << 33;
const BIT_ALIAS_TXT: u64 = 1 << 34;
const BIT_HANGUL: u64 = 1 << 35;
const BIT_IDEOGRAPH: u64 = 1 << 36;

#[derive(Clone, IntoValue, Debug)]
pub struct UnicodeCharNameEntry {
    pub character: char,
    pub codepoint: u32,
    pub name: &'static str,
    pub unicode_data_txt: bool,
    pub alias: bool,
    pub hangul: bool,
    pub ideograph: bool,
}

#[derive(Clone, IntoValue, Debug)]
pub struct UnicodeCharName {
    pub character: char,
    pub codepoint: u32,
    // some characters do not have names, only aliases or properties
    pub name: Option<&'static str>,
    pub aliases: Vec<&'static str>,
    pub hangul: bool,
    pub ideograph: bool,
}

pub static NAMES: LazyLock<HashMap<char, UnicodeCharName>> = LazyLock::new(|| {
    generated::NAMES
        .iter()
        .filter_map(|(name, codepoint)| {
            let codepoint_bits = (*codepoint & BIT_CODEPOINT_MASK) as u32;
            let ch = char::from_u32(codepoint_bits)?;

            let unicode_data_txt = (*codepoint & BIT_UNICODE_DATA_TXT) != 0;
            let alias = (*codepoint & BIT_ALIAS_TXT) != 0;
            let hangul = (*codepoint & BIT_HANGUL) != 0;
            let ideograph = (*codepoint & BIT_IDEOGRAPH) != 0;

            Some(UnicodeCharNameEntry {
                character: ch,
                codepoint: codepoint_bits,
                name,
                unicode_data_txt,
                alias,
                hangul,
                ideograph,
            })
        })
        .fold(HashMap::new(), |mut result, name_entry| {
            if tracing::enabled!(tracing::Level::DEBUG) {
                tracing::debug!(phase = "init", ch = name_entry.character as u32, name = ?name_entry);
            }

            let name = result
                .entry(name_entry.character)
                .or_insert_with(|| {
                    let (name, aliases) = match name_entry.alias {
                        true => (None, vec![name_entry.name]),
                        false => (Some(name_entry.name), vec![]),
                    };

                    UnicodeCharName {
                        character: name_entry.character,
                        codepoint: name_entry.codepoint,
                        name,
                        aliases,
                        hangul: name_entry.hangul,
                        ideograph: name_entry.ideograph,
                    }
                });

            if name_entry.alias {
                name.aliases.push(name_entry.name);

            // we saw an alias first, so set the primary name
            } else {
                if name.name.is_some_and(|n| n != name_entry.name) {
                    tracing::warn!("duplicate name: {:#?}, existing: {:#?}", name_entry, name.name.unwrap());
                }

                name.name = Some(name_entry.name);
            }

            result
        })
});
