use ucd_parse::{
    Codepoint, UnicodeData, UnicodeDataDecomposition, UnicodeDataDecompositionTag,
    UnicodeDataNumeric,
};

pub struct UnicodeDataDecompositionTagLiteral(pub UnicodeDataDecompositionTag);

impl std::fmt::Debug for UnicodeDataDecompositionTagLiteral {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_fmt(format_args!(
            "UnicodeDataDecompositionTagStatic(UnicodeDataDecompositionTag::{:?})",
            self.0
        ))
    }
}

pub struct UnicodeDataNumericLiteral(pub UnicodeDataNumeric);

impl std::fmt::Debug for UnicodeDataNumericLiteral {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_fmt(format_args!(
            "UnicodeDataNumericStatic(UnicodeDataNumeric::{:?})",
            self.0
        ))
    }
}

pub struct UnicodeDataDecompositionLiteral<'a>(pub &'a UnicodeDataDecomposition);

impl std::fmt::Debug for UnicodeDataDecompositionLiteral<'_> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("UnicodeDataDecompositionStatic")
            .field(
                "tag",
                &self
                    .0
                    .tag
                    .as_ref()
                    .map(|tag| UnicodeDataDecompositionTagLiteral(tag.clone())),
            )
            .field("len", &self.0.len)
            .field(
                "mapping",
                &format_args!(
                    "&{:?}",
                    self.0
                        .mapping
                        .iter()
                        .map(|cp| cp.value())
                        .take_while(|cp| *cp != 0)
                        .collect::<Vec<_>>()
                        .as_slice()
                ),
            )
            .finish()
    }
}

pub struct UnicodeDataLiteral(pub UnicodeData);

impl std::fmt::Debug for UnicodeDataLiteral {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let zero = Codepoint::from_u32(0).unwrap();

        let default_mapping = [
            self.0.codepoint,
            zero,
            zero,
            zero,
            zero,
            zero,
            zero,
            zero,
            zero,
            zero,
            zero,
            zero,
            zero,
            zero,
            zero,
            zero,
            zero,
            zero,
        ];

        let decomposition = if self.0.decomposition.mapping == default_mapping
            && self.0.decomposition.tag.is_none()
        {
            None
        } else {
            Some(UnicodeDataDecompositionLiteral(&self.0.decomposition))
        };

        fmt.debug_struct("UnicodeDataStatic")
            .field("codepoint", &self.0.codepoint.value())
            .field("name", &self.0.name)
            .field("general_category", &self.0.general_category)
            .field(
                "canonical_combining_class",
                &self.0.canonical_combining_class,
            )
            .field("bidi_class", &self.0.bidi_class)
            .field("decomposition", &decomposition)
            .field("numeric_type_decimal", &self.0.numeric_type_decimal)
            .field("numeric_type_digit", &self.0.numeric_type_digit)
            .field(
                "numeric_type_numeric",
                &self
                    .0
                    .numeric_type_numeric
                    .as_ref()
                    .map(|num| UnicodeDataNumericLiteral(*num)),
            )
            .field("bidi_mirrored", &self.0.bidi_mirrored)
            .field("unicode1_name", &self.0.unicode1_name)
            .field("iso_comment", &self.0.iso_comment)
            .field(
                "simple_uppercase_mapping",
                &self
                    .0
                    .simple_uppercase_mapping
                    .map(ucd_parse::Codepoint::value),
            )
            .field(
                "simple_lowercase_mapping",
                &self
                    .0
                    .simple_lowercase_mapping
                    .map(ucd_parse::Codepoint::value),
            )
            .field(
                "simple_titlecase_mapping",
                &self
                    .0
                    .simple_titlecase_mapping
                    .map(ucd_parse::Codepoint::value),
            )
            .finish()
    }
}
