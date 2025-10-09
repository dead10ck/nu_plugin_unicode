pub mod commands {
    pub mod chars {
        pub const NAME: &str = "unicode chars";

        pub mod flags {
            pub const ENCODING: &str = "encoding";
            pub const IGNORE_BOM: &str = "ignore-bom";
        }

        pub mod defaults {
            pub const ENCODING: &str = "utf8";
        }
    }
}
