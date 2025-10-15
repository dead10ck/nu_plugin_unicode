use crate::types::unicode_data::{
    UnicodeDataDecompositionStatic, UnicodeDataDecompositionTagStatic, UnicodeDataNumericStatic,
    UnicodeDataStatic,
};

include!(concat!(env!("OUT_DIR"), "/unicode_data.rs"));
