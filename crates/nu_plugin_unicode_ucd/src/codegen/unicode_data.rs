use crate::types::{
    UnicodeDataDecompositionStatic, UnicodeDataDecompositionTagStatic, UnicodeDataNumericStatic,
    UnicodeDataStatic,
};

include!(concat!(env!("OUT_DIR"), "/unicode_data.rs"));
