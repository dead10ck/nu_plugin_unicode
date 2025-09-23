use std::{collections::HashMap, sync::LazyLock};

mod names;

pub static NAMES: LazyLock<HashMap<char, &'static str>> = LazyLock::new(|| {
    names::NAMES
        .iter()
        .filter_map(|(name, codepoint)| {
            let ch = char::from_u32(*codepoint)?;
            Some((ch, *name))
        })
        .collect()
});
