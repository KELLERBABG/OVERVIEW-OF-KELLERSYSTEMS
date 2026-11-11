#!/usr/bin/env bash
# Stage a relocatable Keller-Verify release archive.
#
# The archive is *not* a statically linked binary. The verifier reads MIR through
# `rustc_private` and loads the pinned toolchain's driver libraries at run time,
# so a release bundles the executable, the Z3 library it linked against, and a
# launcher that puts both on the loader path. The pinned Rust toolchain must be
# installed on the target machine (`docs/RELEASE.md` says which one); the
# launcher resolves it through rustup and the verifier checks its commit before
# the run starts.
#
# Usage:
#   Z3_HOME=/path/to/z3 scripts/package-release.sh [output-dir]
#
# Environment:
#   Z3_HOME            Z3 install root (bin/ + include/z3.h). Defaults to
#                      <repo>/.z3/current when it exists.
#   KELLER_CARGO_TARGET_DIR  Cargo target directory. Defaults to <repo>/target.
#   KELLER_TARGET      Host triple to package. Defaults to the toolchain's own.
set -euo pipefail

readonly REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly OUT="${1:-$REPO/dist}"
readonly CHANNEL="$(sed -n 's/^channel = "\(.*\)"/\1/p' "$REPO/rust-toolchain.toml" | head -n1)"
readonly VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$REPO/Cargo.toml" | head -n1)"

fail() { printf '[keller-package] ERROR: %s\n' "$*" >&2; exit 1; }

[[ -n "$CHANNEL" ]] || fail "could not read the pinned channel from rust-toolchain.toml"
[[ -n "$VERSION" ]] || fail "could not read the package version from Cargo.toml"
command -v rustup >/dev/null || fail "rustup is required to resolve the pinned toolchain"

# Ask rustup which toolchain cargo actually used, in the repository, rather than
# resolving the bare channel here. With both host builds installed, a bare
# `--toolchain nightly-...` can select a different host than the active override;
# on Windows that packaged a GNU binary as an MSVC bundle.
readonly ACTIVE="$(cd "$REPO" && rustup show active-toolchain | awk '{print $1; exit}')"
[[ -n "$ACTIVE" ]] || fail "rustup reports no active toolchain; run 'rustup default $CHANNEL' or add a rust-toolchain.toml override"
case "$ACTIVE" in
  "$CHANNEL"|"$CHANNEL"-*) ;;
  *) fail "the active toolchain '$ACTIVE' is not the pinned channel '$CHANNEL'; packaging would bundle a different compiler" ;;
esac
readonly TOOLCHAIN="$ACTIVE"
readonly RUSTC="$(rustup which --toolchain "$TOOLCHAIN" rustc)"
readonly TARGET="${KELLER_TARGET:-$("$RUSTC" -vV | sed -n 's/^host: //p')}"
[[ -n "$TARGET" ]] || fail "could not determine the host triple from rustc -vV"

Z3_HOME="${Z3_HOME:-}"
if [[ -z "$Z3_HOME" && -d "$REPO/.z3/current" ]]; then
  Z3_HOME="$REPO/.z3/current"
fi
[[ -n "$Z3_HOME" && -d "$Z3_HOME" ]] || fail "Z3_HOME is not set and no .z3/current exists"
[[ -f "$Z3_HOME/include/z3.h" ]] || fail "Z3_HOME has no include/z3.h: $Z3_HOME"
if [[ -f "$Z3_HOME/bin/libz3.so" ]]; then
  z3_lib="$Z3_HOME/bin/libz3.so"
elif [[ -f "$Z3_HOME/bin/libz3.dylib" ]]; then
  z3_lib="$Z3_HOME/bin/libz3.dylib"
else
  fail "Z3_HOME has no libz3 shared library in bin/: $Z3_HOME"
fi

readonly TARGET_DIR="${KELLER_CARGO_TARGET_DIR:-$REPO/target}"
binary=''
for candidate in \
  "$TARGET_DIR/$TARGET/release/keller-verify" \
  "$TARGET_DIR/release/keller-verify"; do
  if [[ -x "$candidate" ]]; then binary="$candidate"; break; fi
done
[[ -n "$binary" ]] || fail "no release build at $TARGET_DIR/$TARGET/release/keller-verify or $TARGET_DIR/release/keller-verify; run 'cargo build --release --locked' first"

readonly NAME="keller-verify-$VERSION-$TARGET"
readonly STAGE="$OUT/$NAME"
rm -rf -- "$STAGE"
mkdir -p "$STAGE/bin" "$STAGE/z3/bin" "$STAGE/z3/include" "$STAGE/docs"

for exe in keller-verify keller-env cargo-keller; do
  src="$(dirname "$binary")/$exe"
  [[ -x "$src" ]] || fail "release build did not produce $exe beside keller-verify"
  install -m 0755 "$src" "$STAGE/bin/$exe"
done

cp "$Z3_HOME"/bin/* "$STAGE/z3/bin/"
cp "$Z3_HOME/include/z3.h" "$STAGE/z3/include/"
[[ -f "$Z3_HOME/.keller-sha256" ]] && cp "$Z3_HOME/.keller-sha256" "$STAGE/z3/SHA256"
(cd "$STAGE/z3/bin" && sha256sum "$(basename "$z3_lib")" > SHA256SUMS)

cp "$REPO/README.md" "$STAGE/README.md"
cp "$REPO/LICENSE-MIT" "$STAGE/LICENSE-MIT"
cp "$REPO/LICENSE-APACHE" "$STAGE/LICENSE-APACHE"
cp "$REPO/docs/RELEASE.md" "$STAGE/docs/RELEASE.md"

# The launcher is generated here, not committed, because it names this bundle's
# layout. It resolves the pinned toolchain at run time so the archive can be
# unpacked anywhere; the verifier then enforces the exact commit.
cat > "$STAGE/bin/keller" <<EOF
#!/usr/bin/env bash
set -euo pipefail
here="\$(cd "\$(dirname "\${BASH_SOURCE[0]}")" && pwd)"
root="\$(dirname "\$here")"
toolchain='$TOOLCHAIN'
rustc="\$(rustup which --toolchain "\$toolchain" rustc)"
host="\$("\$rustc" -vV | sed -n 's/^host: //p')"
rustlib="\$(dirname "\$rustc")/../lib"
driver="\$rustlib/rustlib/\$host/lib"
[[ -d "\$driver" ]] || { echo "keller: pinned rustc driver libraries missing: \$driver" >&2; exit 3; }
export RUSTUP_TOOLCHAIN="\$toolchain"
export Z3_HOME="\$root/z3"
export Z3_SYS_Z3_HEADER="\$root/z3/include/z3.h"
export PATH="\$driver:\$root/z3/bin:\$PATH"
export LD_LIBRARY_PATH="\$driver:\$rustlib:\$root/z3/bin\${LD_LIBRARY_PATH:+:\$LD_LIBRARY_PATH}"
export DYLD_LIBRARY_PATH="\$driver:\$rustlib:\$root/z3/bin\${DYLD_LIBRARY_PATH:+:\$DYLD_LIBRARY_PATH}"
exec "\$here/keller-verify" "\$@"
EOF
chmod 0755 "$STAGE/bin/keller"

cat > "$STAGE/manifest.json" <<EOF
{
  "tool": "keller-verify",
  "version": "$VERSION",
  "target": "$TARGET",
  "rustc": {
    "release": "$("$RUSTC" -vV | sed -n 's/^release: //p')",
    "commit": "$("$RUSTC" -vV | sed -n 's/^commit-hash: //p')",
    "host": "$("$RUSTC" -vV | sed -n 's/^host: //p')"
  },
  "toolchain": "$TOOLCHAIN",
  "z3_version": "$("$Z3_HOME/bin/z3" --version 2>/dev/null || echo unknown)",
  "z3_sha256": "$(sha256sum "$z3_lib" | cut -d' ' -f1)",
  "runtime": "requires the pinned Rust toolchain (rustc-dev, llvm-tools-preview); Z3 is bundled"
}
EOF

mkdir -p "$OUT"
archive="$OUT/$NAME.tar.gz"
tar -czf "$archive" -C "$OUT" "$NAME"
# Record the bare file name, not the absolute path it happened to be built at:
# the release job re-checks the combined SHA256SUMS on a different runner, in a
# different checkout directory, where an absolute path would not resolve.
(cd "$OUT" && sha256sum "$NAME.tar.gz" > "$NAME.tar.gz.sha256")

printf '[keller-package] %s\n' "$archive"
printf '[keller-package] %s\n' "$archive.sha256"
