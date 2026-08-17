//! Makes the Rust snippets in `docs/content/` fail the build when they rot.
//!
//! The site renders that Markdown; nothing compiled it. Every Rust block was
//! marked ```` ```rust,ignore ````, which is why four how-to pages could teach
//! `rumi eval`, `rumi validate` and `register_input` — none of which ever
//! existed — without anything noticing.
//!
//! `#[doc = include_str!(...)]` hands each page to rustdoc, so `cargo test
//! --doc` compiles its Rust blocks. Blocks marked `no_run` are compiled and not
//! executed, which is right for snippets that read a config file from disk.
//!
//! This crate exists rather than the includes living in `rumi-core` because
//! `include_str!` needs the file present at build time, and `docs/` is not part
//! of a published crate's package. Putting them here keeps `cargo publish`
//! working. It is `publish = false` and has no runtime code.

#![cfg(doctest)]

macro_rules! doc_page {
    ($name:ident, $path:literal) => {
        #[doc = include_str!($path)]
        pub struct $name;
    };
}

doc_page!(
    GettingStartedRust,
    "../../../docs/content/getting-started/rust.md"
);
doc_page!(
    ConceptsPipeline,
    "../../../docs/content/concepts/pipeline.md"
);
doc_page!(
    ConceptsTypeErasure,
    "../../../docs/content/concepts/type-erasure.md"
);
