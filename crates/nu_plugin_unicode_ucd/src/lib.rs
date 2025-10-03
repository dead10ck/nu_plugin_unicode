use crate::types::{UnicodeDataDecompositionStatic, UnicodeDataStatic};

mod types;

include!(concat!(env!("OUT_DIR"), "/codegen.rs"));
