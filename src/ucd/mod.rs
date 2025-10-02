use crate::ucd::types::{UnicodeDataDecompositionStatic, UnicodeDataStatic};

mod types;

include!(concat!(env!("OUT_DIR"), "/codegen.rs"));
