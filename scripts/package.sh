#!/bin/sh
# Run after the Rust release build and frontend build. No credentials are packed.
set -eu
project_dir=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
output_dir=${1:-"$project_dir/dist"}
binary_path=${2:-"$project_dir/target/x86_64-unknown-linux-musl/release/ruoyi-rust-by"}
test -f "$binary_path"
test -f "$project_dir/frontend/dist/index.html"
mkdir -p "$output_dir"
output_dir=$(CDPATH= cd -- "$output_dir" && pwd)
package_dir=$(mktemp -d)
trap 'rm -rf "$package_dir"' EXIT HUP INT TERM
mkdir -p "$package_dir/public/admin" "$package_dir/uploads"
cp "$binary_path" "$package_dir/ruoyi-rust-by"
cp -a "$project_dir/frontend/dist/." "$package_dir/public/admin/"
cp "$project_dir/.env.example" "$project_dir/LICENSE" "$project_dir/THIRD_PARTY_NOTICES.md" "$project_dir/README.md" "$package_dir/"
cp -a "$project_dir/deploy" "$project_dir/docs" "$package_dir/"
tar -C "$package_dir" -czf "$output_dir/ruoyi-rust-by-v0.1.0-linux-x86_64.tar.gz" .
(cd "$output_dir" && sha256sum ruoyi-rust-by-v0.1.0-linux-x86_64.tar.gz > SHA256SUMS)
printf 'Package: %s\n' "$output_dir/ruoyi-rust-by-v0.1.0-linux-x86_64.tar.gz"
