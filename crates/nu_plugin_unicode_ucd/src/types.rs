//! These types are copied from the ucd-parse crate, but changed to have static
//! types.

use nu_protocol::{IntoValue, Span, Value, record};
use ucd_parse::{UnicodeDataDecompositionTag, UnicodeDataNumeric};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnicodeDataDecompositionTagStatic(pub ucd_parse::UnicodeDataDecompositionTag);

impl IntoValue for UnicodeDataDecompositionTagStatic {
    fn into_value(self, span: Span) -> Value {
        match self.0 {
            UnicodeDataDecompositionTag::Font => "font",
            UnicodeDataDecompositionTag::NoBreak => "noBreak",
            UnicodeDataDecompositionTag::Initial => "initial",
            UnicodeDataDecompositionTag::Medial => "medial",
            UnicodeDataDecompositionTag::Final => "final",
            UnicodeDataDecompositionTag::Isolated => "isolated",
            UnicodeDataDecompositionTag::Circle => "circle",
            UnicodeDataDecompositionTag::Super => "super",
            UnicodeDataDecompositionTag::Sub => "sub",
            UnicodeDataDecompositionTag::Vertical => "vertical",
            UnicodeDataDecompositionTag::Wide => "wide",
            UnicodeDataDecompositionTag::Narrow => "narrow",
            UnicodeDataDecompositionTag::Small => "small",
            UnicodeDataDecompositionTag::Square => "square",
            UnicodeDataDecompositionTag::Fraction => "fraction",
            UnicodeDataDecompositionTag::Compat => "compat",
        }
        .into_value(span)
    }
}

/// Represents a decomposition mapping of a single row in the
/// `UnicodeData.txt` file.
#[derive(Clone, Debug, Default, Eq, PartialEq, IntoValue)]
pub struct UnicodeDataDecompositionStatic {
    /// The formatting tag associated with this mapping, if present.
    pub tag: Option<UnicodeDataDecompositionTagStatic>,
    /// The number of codepoints in this mapping.
    pub len: u32,
    /// The codepoints in the mapping. Entries beyond `len` in the mapping
    /// are always U+0000. If no mapping was present, then this always contains
    /// a single codepoint corresponding to this row's character.
    pub mapping: [u32; 18],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnicodeDataNumericStatic(pub ucd_parse::UnicodeDataNumeric);

impl IntoValue for UnicodeDataNumericStatic {
    fn into_value(self, span: Span) -> Value {
        match self.0 {
            UnicodeDataNumeric::Integer(val) => record!("integer" => val.into_value(span)),
            UnicodeDataNumeric::Rational(num, denom) => record!(
                "numerator" => num.into_value(span),
                "denominator" => denom.into_value(span),
            ),
        }
        .into_value(span)
    }
}

/// Represents a single row in the `UnicodeData.txt` file.
///
/// These fields were taken from UAX44, Table 9, as part of the documentation
/// for the
/// [`UnicodeData.txt` file](https://www.unicode.org/reports/tr44/#UnicodeData.txt).
#[derive(Clone, Debug, Default, Eq, PartialEq, IntoValue)]
pub struct UnicodeDataStatic {
    /// The codepoint corresponding to this row.
    pub codepoint: u32,
    /// The name of this codepoint.
    pub name: &'static str,
    /// The "general category" of this codepoint.
    pub general_category: &'static str,
    /// The class of this codepoint used in the Canonical Ordering Algorithm.
    ///
    /// Note that some classes map to a particular symbol. See
    /// [UAX44, Table 15](https://www.unicode.org/reports/tr44/#Canonical_Combining_Class_Values).
    pub canonical_combining_class: u8,
    /// The bidirectional class of this codepoint.
    ///
    /// Possible values are listed in
    /// [UAX44, Table 13](https://www.unicode.org/reports/tr44/#Bidi_Class_Values).
    pub bidi_class: &'static str,
    /// The decomposition mapping for this codepoint. This includes its
    /// formatting tag (if present).
    pub decomposition: UnicodeDataDecompositionStatic,
    /// A decimal numeric representation of this codepoint, if it has the
    /// property `Numeric_Type=Decimal`.
    pub numeric_type_decimal: Option<u8>,
    /// A decimal numeric representation of this codepoint, if it has the
    /// property `Numeric_Type=Digit`. Note that while this field is still
    /// populated for existing codepoints, no new codepoints will have this
    /// field populated.
    pub numeric_type_digit: Option<u8>,
    /// A decimal or rational numeric representation of this codepoint, if it
    /// has the property `Numeric_Type=Numeric`.
    pub numeric_type_numeric: Option<UnicodeDataNumericStatic>,
    /// A boolean indicating whether this codepoint is "mirrored" in
    /// bidirectional text.
    pub bidi_mirrored: bool,
    /// The "old" Unicode 1.0 or ISO 6429 name of this codepoint. Note that
    /// this field is empty unless it is significantly different from
    /// the `name` field.
    pub unicode1_name: &'static str,
    /// The ISO 10464 comment field. This no longer contains any non-NULL
    /// values.
    pub iso_comment: &'static str,
    /// This codepoint's simple uppercase mapping, if it exists.
    pub simple_uppercase_mapping: Option<u32>,
    /// This codepoint's simple lowercase mapping, if it exists.
    pub simple_lowercase_mapping: Option<u32>,
    /// This codepoint's simple titlecase mapping, if it exists.
    pub simple_titlecase_mapping: Option<u32>,
}
