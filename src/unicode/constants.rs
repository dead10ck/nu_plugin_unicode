pub mod commands {
    use nu_protocol::SyntaxShape;

    pub struct Flag {
        pub name: &'static str,
        pub short: Option<char>,
        pub shape: Option<SyntaxShape>,
        pub desc: &'static str,
    }

    pub mod ucd {
        pub mod index {
            pub const DATA_DIR_NAME: &str = "nu_plugin_unicode";

            pub mod common {
                use crate::unicode::constants::commands::Flag;
                use nu_protocol::SyntaxShape;

                pub static INDEX_DIR: Flag = Flag {
                    name: "index-dir",
                    short: Some('i'),
                    shape: Some(SyntaxShape::Filepath),
                    desc: "Directory in which to save indexed UCD data",
                };
            }

            pub mod dataframe {
                pub mod table {
                    pub mod unicode_data {
                        pub const NAME: &str = "unicode_data";

                        pub mod fields {
                            pub const CODEPOINT: &str = "codepoint";
                            pub const NAME: &str = "name";
                            pub const GENERAL_CATEGORY: &str = "general_category";
                            pub const CANONICAL_COMBINING_CLASS: &str = "canonical_combining_class";
                            pub const BIDI_CLASS: &str = "bidi_class";
                            pub const DECOMPOSITION: &str = "decomposition";
                            pub const NUMERIC_TYPE_DECIMAL: &str = "numeric_type_decimal";
                            pub const NUMERIC_TYPE_DIGIT: &str = "numeric_type_digit";
                            pub const NUMERIC_TYPE_NUMERIC: &str = "numeric_type_numeric";
                            pub const BIDI_MIRRORED: &str = "bidi_mirrored";
                            pub const UNICODE1_NAME: &str = "unicode1_name";
                            pub const ISO_COMMENT: &str = "iso_comment";
                            pub const SIMPLE_UPPERCASE_MAPPING: &str = "simple_uppercase_mapping";
                            pub const SIMPLE_LOWERCASE_MAPPING: &str = "simple_lowercase_mapping";
                            pub const SIMPLE_TITLECASE_MAPPING: &str = "simple_titlecase_mapping";
                        }
                    }
                }

                pub mod persist {
                    pub mod path {
                        pub const UNICODE_DATA: &str = "unicode_data.parquet";
                    }
                }
            }

            pub mod build {
                pub const NAME: &str = "unicode ucd index build";

                pub mod flags {
                    use crate::unicode::constants::commands::{Flag, ucd::index};
                    use nu_protocol::SyntaxShape;

                    pub static UCD_DIR: Flag = Flag {
                        name: "ucd-dir",
                        short: Some('u'),
                        shape: Some(SyntaxShape::Filepath),
                        desc: "Directory with the contents of the unarchived UCD.zip",
                    };

                    pub static FLAGS: &[&Flag] = &[&index::common::INDEX_DIR, &UCD_DIR];
                }

                pub mod defaults {}
            }
        }
    }

    pub mod chars {
        pub const NAME: &str = "unicode chars";

        pub mod flags {
            use crate::unicode::constants::commands::{Flag, ucd::index};
            use nu_protocol::SyntaxShape;

            pub static ENCODING: Flag = Flag {
                name: "encoding",
                short: Some('e'),
                shape: Some(SyntaxShape::String),
                desc: "Encoding of the input bytes. By default, BOM sniffing occurs to detect the encoding; failing that, UTF-8 is assumed.",
            };

            pub static IGNORE_BOM: Flag = Flag {
                name: "ignore-bom",
                short: Some('b'),
                shape: None,
                desc: "Ignore the BOM, if present. By default, even if an encoding is specified, if a BOM is present, the encoding from the command line is ignored.",
            };

            pub static FLAGS: &[&Flag] = &[&ENCODING, &IGNORE_BOM, &index::common::INDEX_DIR];
        }

        pub mod defaults {
            pub const ENCODING: &str = "utf8";
        }
    }
}
