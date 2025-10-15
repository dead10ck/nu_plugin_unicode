use nu_protocol::{IntoValue, Span, Value};
use ucd_parse::NameAliasLabel;

/// A single row in the `NameAliases.txt` file.
///
/// Note that there are multiple rows for some codepoint. Each row provides a
/// new alias.
#[derive(Clone, Debug, Default, Eq, PartialEq, IntoValue)]
pub struct NameAliasStatic {
    /// The codepoint corresponding to this row.
    pub codepoint: u32,
    /// The alias.
    pub alias: &'static str,
    /// The label of this alias.
    pub label: NameAliasLabelStatic,
}

/// The label of a name alias.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NameAliasLabelStatic(pub ucd_parse::NameAliasLabel);

impl IntoValue for NameAliasLabelStatic {
    fn into_value(self, span: Span) -> Value {
        match self.0 {
            NameAliasLabel::Correction => "correction",
            NameAliasLabel::Control => "control",
            NameAliasLabel::Alternate => "alternate",
            NameAliasLabel::Figment => "figment",
            NameAliasLabel::Abbreviation => "abbreviation",
        }
        .into_value(span)
    }
}
