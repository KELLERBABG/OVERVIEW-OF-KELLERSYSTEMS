#!/usr/bin/env bash
set -euo pipefail

readonly TOOLCHAIN=nightly-2026-07-24
readonly HOST=x86_64-unknown-linux-gnu
readonly Z3_VERSION=4.16.0
readonly Z3_ARCHIVE="z3-${Z3_VERSION}-x64-glibc-2.39.zip"
readonly Z3_ROOT="z3-${Z3_VERSION}-x64-glibc-2.39"
readonly Z3_SHA256=7288c49a5bd6dbafd7b0b0d1f65956b91672da24b08f09242919af159be3418e
readonly REPOSITORY="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly PREFIX="${KELLER_PREFIX:-$HOME/.local}"
readonly DATA="$PREFIX/share/keller-verify"
readonly BIN="$PREFIX/bin"
readonly INSTALL="$PREFIX/lib/keller-verify"
readonly Z3_HOME="$DATA/$Z3_ROOT"
readonly Z3_DIGEST_MARKER="$Z3_HOME/.keller-sha256"
readonly CARGO="$(command -v cargo || true)"
readonly RUSTUP="$(command -v rustup || true)"

fail() { printf '[keller-install] ERROR: %s\n' "$*" >&2; exit 1; }
[[ "$PREFIX" == /* ]] || fail "KELLER_PREFIX must be absolute"
export CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
export RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"
[[ -n "$CARGO" ]] || fail "cargo is required; install Rust with rustup and ensure cargo is on PATH"
[[ -n "$RUSTUP" ]] || fail "rustup is required; install it from https://rustup.rs/ and ensure rustup is on PATH"
[[ "$(uname -s)" == Linux && "$(uname -m)" == x86_64 ]] || fail "only Ubuntu 24.04 x86_64 is supported"
[[ -r /etc/os-release ]] || fail "cannot identify Linux distribution"
. /etc/os-release
[[ "${ID:-}" == ubuntu && "${VERSION_ID:-}" == 24.04 ]] || fail "only Ubuntu 24.04 is supported"
missing=()
for tool in curl sha256sum clang getconf unzip dirname mktemp install; do
  command -v "$tool" >/dev/null || missing+=("$tool")
done
libclang_found=0
if [[ -n "${LIBCLANG_PATH:-}" ]]; then
  for candidate in "$LIBCLANG_PATH"/libclang*.so*; do
    if [[ -e "$candidate" ]]; then libclang_found=1; break; fi
  done
fi
if (( libclang_found == 0 )); then
  for candidate in /usr/lib/llvm-*/lib/libclang-*.so* /usr/lib/x86_64-linux-gnu/libclang-*.so* /usr/lib/clang/*/lib/libclang*.so*; do
    if [[ -e "$candidate" ]]; then libclang_found=1; break; fi
  done
fi
(( libclang_found == 1 )) || missing+=("libclang-dev (libclang shared library)")
if (( ${#missing[@]} > 0 )); then
  printf -v missing_list '%s, ' "${missing[@]}"
  missing_list="${missing_list%, }"
  fail "missing prerequisites: $missing_list; on Ubuntu install with 'sudo apt-get install build-essential clang libclang-dev curl unzip'"
fi
[[ "$(getconf GNU_LIBC_VERSION)" == 'glibc 2.39' ]] || fail "glibc 2.39 is required"

mkdir -p "$DATA" "$BIN" "$INSTALL"
target=''
work=''
installed_binary=''
launcher=''
cleanup() {
  [[ -z "$target" ]] || rm -rf -- "$target"
  [[ -z "$work" ]] || rm -rf -- "$work"
  [[ -z "$installed_binary" ]] || rm -f -- "$installed_binary"
  [[ -z "$launcher" ]] || rm -f -- "$launcher"
}
trap cleanup EXIT
target="$(mktemp -d "${TMPDIR:-/tmp}/keller-build.XXXXXXXX")"
work="$(mktemp -d "${TMPDIR:-/tmp}/keller-z3.XXXXXXXX")"


if [[ -e "$Z3_HOME" ]]; then
  [[ -x "$Z3_HOME/bin/z3" && -f "$Z3_HOME/bin/libz3.so" &&
     -f "$Z3_HOME/include/z3.h" && -f "$Z3_DIGEST_MARKER" &&
     "$("$Z3_HOME/bin/z3" -version 2>/dev/null || true)" == *"$Z3_VERSION"* &&
     "$(cat "$Z3_DIGEST_MARKER" 2>/dev/null || true)" == "$Z3_SHA256" ]] ||
    fail "existing Z3 install at '$Z3_HOME' is incomplete or unverified; move it aside and retry"
else
  printf '[keller-install] Fetching pinned Z3 %s...\n' "$Z3_VERSION"
  curl --fail --location --silent --show-error --retry 3 \
    "https://github.com/Z3Prover/z3/releases/download/z3-$Z3_VERSION/$Z3_ARCHIVE" \
    --output "$work/$Z3_ARCHIVE"
  printf '%s  %s\n' "$Z3_SHA256" "$work/$Z3_ARCHIVE" | sha256sum --check --status || fail "Z3 SHA-256 mismatch"
  unzip -q "$work/$Z3_ARCHIVE" -d "$work/unpacked"
  extracted="$work/unpacked/$Z3_ROOT"
  [[ -x "$extracted/bin/z3" && -f "$extracted/bin/libz3.so" && -f "$extracted/include/z3.h" ]] || fail "unexpected Z3 archive layout"
  z3_version="$("$extracted/bin/z3" -version)"
  [[ "$z3_version" == *"$Z3_VERSION"* ]] || fail "unexpected Z3 version: $z3_version"
  printf '%s\n' "$Z3_SHA256" > "$extracted/.keller-sha256"
  mv "$extracted" "$Z3_HOME"
fi

export Z3_HOME Z3_SYS_Z3_HEADER="$Z3_HOME/include/z3.h"
export PATH="$Z3_HOME/bin:$PATH"
"$RUSTUP" toolchain install "$TOOLCHAIN" --profile minimal --target "$HOST" --component rustc-dev --component llvm-tools-preview
TOOLCHAIN_BIN="$(dirname "$("$RUSTUP" which --toolchain "$TOOLCHAIN" rustc)")"
TOOLCHAIN_RUSTLIB="$TOOLCHAIN_BIN/../lib/rustlib/$HOST/lib"

printf '[keller-install] Building locked release for %s...\n' "$HOST"
(
  cd "$REPOSITORY"
  "$CARGO" "+$TOOLCHAIN" build --locked --release --bin keller-verify \
    --target "$HOST" --target-dir "$target"
)
binary="$target/$HOST/release/keller-verify"
[[ -x "$binary" ]] || fail "release binary missing after build"
export LD_LIBRARY_PATH="$target/$HOST/release/deps:$TOOLCHAIN_RUSTLIB:$TOOLCHAIN_BIN/../lib:$Z3_HOME/bin${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
"$binary" --version >/dev/null
installed_binary="$(mktemp "$INSTALL/.keller-verify.XXXXXXXX")"
install -m 0755 "$binary" "$installed_binary"
mv -f "$installed_binary" "$INSTALL/keller-verify"

# Quote custom prefixes as Bash words before embedding them in the launcher.
printf -v q_toolchain '%q' "$TOOLCHAIN"
printf -v q_host '%q' "$HOST"
printf -v q_install '%q' "$INSTALL"
printf -v q_z3 '%q' "$Z3_HOME"
printf -v q_rustup '%q' "$RUSTUP"
launcher="$(mktemp "$BIN/.keller-verify.XXXXXXXX")"
cat > "$launcher" <<EOF
#!/usr/bin/env bash
set -euo pipefail
readonly TOOLCHAIN=$q_toolchain
readonly HOST=$q_host
readonly INSTALL=$q_install
readonly Z3_HOME=$q_z3
readonly RUSTUP=$q_rustup
readonly RUSTC="\$("\$RUSTUP" which --toolchain "\$TOOLCHAIN" rustc)"
readonly RUSTLIB="\$(dirname "\$RUSTC")/../lib/rustlib/\$HOST/lib"
readonly RUSTLIB_BIN="\$(dirname "\$RUSTC")/../lib"
[[ -d "\$RUSTLIB" ]] || { echo "keller-verify: pinned rustc driver libraries missing: \$RUSTLIB" >&2; exit 3; }
export RUSTUP_TOOLCHAIN="\$TOOLCHAIN" Z3_HOME Z3_SYS_Z3_HEADER="\$Z3_HOME/include/z3.h"
export PATH="\$RUSTLIB:\$Z3_HOME/bin:\$PATH"
export LD_LIBRARY_PATH="\$RUSTLIB:\$RUSTLIB_BIN:\$Z3_HOME/bin\${LD_LIBRARY_PATH:+:\$LD_LIBRARY_PATH}"
exec "\$INSTALL/keller-verify" "\$@"
EOF
chmod 0755 "$launcher"
mv -f "$launcher" "$BIN/keller-verify"
printf '[keller-install] Installed to %s/keller-verify; add %s to PATH.\n' "$BIN" "$BIN"
