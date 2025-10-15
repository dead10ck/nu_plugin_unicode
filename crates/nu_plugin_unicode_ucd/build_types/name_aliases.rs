use ucd_parse::{NameAlias, NameAliasLabel};

pub struct NameAliasLiteral(pub NameAlias);

impl std::fmt::Debug for NameAliasLiteral {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("NameAliasStatic")
            .field("codepoint", &self.0.codepoint.value())
            .field("alias", &self.0.alias)
            .field("label", &NameAliasLabelLiteral(self.0.label))
            .finish()
    }
}

pub struct NameAliasLabelLiteral(pub NameAliasLabel);

impl std::fmt::Debug for NameAliasLabelLiteral {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_fmt(format_args!(
            "NameAliasLabelStatic(NameAliasLabel::{:?})",
            self.0
        ))
    }
}
