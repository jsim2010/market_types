set shell := ["bash", "-uc"]

alias b := build
alias d := doc
alias f := fix
alias l := lint
alias t := test
alias v := validate

# Ideally `build` would allow warnings - see https://github.com/rust-lang/cargo/issues/3591.
#
# Builds the project for each possible feature combination.
build:
    cargo build
    cargo build --features std
    cargo build --features crossbeam-channel
    cargo build --features crossbeam-queue
    cargo build --features thread
    cargo build --features std,crossbeam-channel
    cargo build --features std,crossbeam-queue
    cargo build --features crossbeam-channel,crossbeam-queue
    cargo build --features crossbeam-channel,thread

# Installs everything needed for dependencies
_install_deps:
    cargo deny --version || cargo install cargo-deny

# Installs everything needed for formatting
_install_format:
    rustup component add rustfmt

# Installs everything needed for linting
_install_lint:
    rustup component add clippy

# Generates documentation for public items
doc:
    cargo +nightly doc --all-features

# Fixes issues that can be addressed automatically
fix: _install_format fix_format

# Formats rust code
fix_format: _install_format
    cargo fmt

# Any lint that is allowed is explained below:
# - box_pointers: box pointers are okay and useful
# - unstable_features: needed for doc_cfg
# - variant_size_differences: handled by clippy::large_enum_variant
# - clippy::missing_inline_in_public_items: compiler already attempts to handle this and there is no check for if a public function is converted to private
# - clippy::module_name_repetitions: repeating the module name in an item can be useful when it clarifies the function of the item
# - clippy::multiple_crate_versions: not fixable when caused by dependencies
# - clippy::implicit_return: rust convention calls for implicit return
# - clippy::redundant_pub_crate: conflicts with unreachable_pub
#
# Lints the project source code
lint: _install_lint
    cargo +nightly clippy --all-features -- \
     -D warnings \
     -D absolute_paths_not_starting_with_crate \
     -D anonymous_parameters \
     -A box_pointers \
     -D deprecated_in_future \
     -D elided_lifetimes_in_paths \
     -D explicit_outlives_requirements \
     -D keyword_idents \
     -D macro_use_extern_crate \
     -D meta_variable_misuse \
     -D missing_copy_implementations \
     -D missing_crate_level_docs \
     -D missing_debug_implementations \
     -D missing_docs \
     -D missing_doc_code_examples \
     -D non_ascii_idents \
     -D pointer_structural_match \
     -D private_doc_tests \
     -D single_use_lifetimes \
     -D trivial_casts \
     -D trivial_numeric_casts \
     -D unaligned_references \
     -D unreachable_pub \
     -D unsafe_code \
     -A unstable_features \
     -D unused_crate_dependencies \
     -D unused_extern_crates \
     -D unused_import_braces \
     -D unused_lifetimes \
     -D unused_qualifications \
     -D unused_results \
     -A variant_size_differences \
     -D clippy::correctness \
     -D clippy::restriction \
     -D clippy::style \
     -D clippy::pedantic \
     -D clippy::complexity \
     -D clippy::perf \
     -D clippy::cargo \
     -D clippy::nursery \
     -A clippy::missing_inline_in_public_items \
     -A clippy::module_name_repetitions \
     -A clippy::multiple_crate_versions \
     -A clippy::implicit_return \
     -A clippy::redundant_pub_crate \

# Configures the version of rust
set_rust version:
    rustup override set {{version}}

# Runs tests
test:
    cargo +nightly test --verbose --all-features

# Validates the project
validate: (set_rust "1.54.0") validate_format validate_deps lint build test validate_doc

# Validates dependencies of the project
validate_deps: _install_deps
    cargo deny check

# Validates the documentation of the project
validate_doc:
    cargo rustdoc -- -D rustdoc::all

# Validates the formatting of the project
validate_format: _install_format
    cargo fmt -- --check
