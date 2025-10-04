use crate::types::{
    UnicodeDataDecompositionStatic, UnicodeDataDecompositionTagStatic, UnicodeDataNumericStatic,
    UnicodeDataStatic,
};

mod types;

include!(concat!(env!("OUT_DIR"), "/codegen.rs"));
