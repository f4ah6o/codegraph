# Cargo release helpers.

crate_manifest := "crates/codegraph/Cargo.toml"

release-check:
    cargo test --all --all-features
    cargo build --release --all-features
    cargo publish --dry-run --locked --manifest-path {{crate_manifest}}

publish-cli: release-check
    cargo publish --locked --manifest-path {{crate_manifest}}

release-tag:
    version=$(rg -n "^version = " {{crate_manifest}} | head -n1 | awk -F'"' '{print $2}'); \
    git tag "v${version}"; \
    git push origin "v${version}"

release: release-check release-tag
