#!/bin/sh
# Run after the Rust release build and frontend build. No credentials are packed.
set -eu
project_dir=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
output_dir=${1:-"$project_dir/dist"}
binary_path=${2:-"$project_dir/target/x86_64-unknown-linux-musl/release/ruoyi-rust-by"}
build_info_path=${3:-}
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$project_dir/Cargo.toml" | head -n 1)
test -n "$version"
archive_name="ruoyi-rust-by-v${version}-linux-x86_64.tar.gz"
test -f "$binary_path"
test -f "$project_dir/frontend/dist/index.html"
mkdir -p "$output_dir"
output_dir=$(CDPATH= cd -- "$output_dir" && pwd)
package_dir=$(mktemp -d)
trap 'rm -rf "$package_dir"' EXIT HUP INT TERM
mkdir -p "$package_dir/public/admin" "$package_dir/uploads"
cp "$binary_path" "$package_dir/ruoyi-rust-by"
cp -a "$project_dir/frontend/dist/." "$package_dir/public/admin/"
cp "$project_dir/.env.example" "$project_dir/LICENSE" "$project_dir/THIRD_PARTY_NOTICES.md" "$project_dir/README.md" "$project_dir/SECURITY.md" "$package_dir/"
cp "$project_dir/frontend/LICENSE" "$package_dir/public/admin/LICENSE"
if [ -n "$build_info_path" ]; then
    cp "$build_info_path" "$package_dir/BUILD_INFO.json"
fi
cp -a "$project_dir/deploy" "$project_dir/docs" "$package_dir/"
tar -C "$package_dir" -czf "$output_dir/$archive_name" .
(cd "$output_dir" && sha256sum "$archive_name" > SHA256SUMS)
printf 'Package: %s\n' "$output_dir/$archive_name"
