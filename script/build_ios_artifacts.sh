#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="${BUILD_DIR:-$ROOT_DIR/build}"
DIST_DIR="${DIST_DIR:-$BUILD_DIR/dist}"

IOS_SDK="${IOS_SDK:-iphonesimulator}"
IOS_DEPLOYMENT_TARGET="${IOS_DEPLOYMENT_TARGET:-16.0}"
BUILD_HOST_OLEANS="${BUILD_HOST_OLEANS:-1}"

case "$IOS_SDK" in
  iphonesimulator)
    IOS_TARGET="${IOS_TARGET:-arm64-apple-ios${IOS_DEPLOYMENT_TARGET}-simulator}"
    ;;
  iphoneos)
    IOS_TARGET="${IOS_TARGET:-arm64-apple-ios${IOS_DEPLOYMENT_TARGET}}"
    ;;
  *)
    : "${IOS_TARGET:?Set IOS_TARGET for IOS_SDK=$IOS_SDK}"
    ;;
esac

SDK_TAG="$(printf '%s' "$IOS_TARGET" | tr '.-' '__')"
SDK_TAG="${SDK_TAG//__/_}"
LEAN_RUNTIME_BUILD_DIR="$BUILD_DIR/lean4-${SDK_TAG}-runtime"
IOS_BUILD_DIR="$BUILD_DIR/ios-lean-${SDK_TAG}"
LIB_DIR="$IOS_BUILD_DIR/lib"
HOST_LEAN_BUILD_DIR="$BUILD_DIR/lean4-host-nogmp"
HOST_LEAN_STAGE1_BUILD_DIR="$HOST_LEAN_BUILD_DIR/stage1"
HOST_OLEAN_DIR="$HOST_LEAN_STAGE1_BUILD_DIR/lib/lean"

IOS_LEANC_WRAPPER="$BUILD_DIR/ios-tooling/ios-leanc.sh"
IOS_AR_WRAPPER="$BUILD_DIR/ios-tooling/ios-ar.sh"
IOS_LEANMAKE="$LEAN_RUNTIME_BUILD_DIR/bin/leanmake"

RUNTIME_LIB="$LEAN_RUNTIME_BUILD_DIR/lib/lean/libleanrt.a"
RUNTIME_CPP_LIB="$LEAN_RUNTIME_BUILD_DIR/lib/lean/libleancpp.a"
RUNTIME_SHELL_LIB="$LEAN_RUNTIME_BUILD_DIR/lib/temp/libleanshell.a"
RUNTIME_INIT_LIB="$LEAN_RUNTIME_BUILD_DIR/lib/temp/libleaninitialize.a"
STDLIB_INIT_LIB="$LIB_DIR/libInit.a"
STDLIB_STD_LIB="$LIB_DIR/libStd.a"
STDLIB_LEAN_LIB="$LIB_DIR/libLean.a"

LEAN_REV="$(git -C "$ROOT_DIR" rev-parse --short=12 HEAD)"
IOS_ARCHIVE="$DIST_DIR/lean4-ios-${SDK_TAG}-${LEAN_REV}.tar.zst"
HOST_ARCHIVE="$DIST_DIR/lean4-host-oleans-macos-${LEAN_REV}.tar.zst"

NPROC="${NPROC:-$(sysctl -n hw.logicalcpu 2>/dev/null || getconf _NPROCESSORS_ONLN 2>/dev/null || echo 4)}"

mkdir -p "$DIST_DIR" "$(dirname "$IOS_LEANC_WRAPPER")"

cat >"$IOS_LEANC_WRAPPER" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

: "${IOS_SDK:?}"
: "${IOS_TARGET:?}"
: "${IOS_DEPLOYMENT_TARGET:?}"
: "${LEAN_RUNTIME_INCLUDE:?}"
: "${LEAN_STAGE0_INCLUDE:?}"
: "${LEAN_SRC_INCLUDE:?}"

SDK_PATH="$(xcrun --sdk "$IOS_SDK" --show-sdk-path)"
CLANG_PATH="$(xcrun --sdk "$IOS_SDK" -f clang)"
CLANG_ARGS=(
  -target "$IOS_TARGET"
  -isysroot "$SDK_PATH"
  -mios-version-min="$IOS_DEPLOYMENT_TARGET"
  -I"$LEAN_RUNTIME_INCLUDE"
  -I"$LEAN_STAGE0_INCLUDE"
  -I"$LEAN_SRC_INCLUDE"
  "$@"
)

exec "$CLANG_PATH" "${CLANG_ARGS[@]}"
EOF

cat >"$IOS_AR_WRAPPER" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

: "${IOS_SDK:?}"

exec xcrun --sdk "$IOS_SDK" ar "$@"
EOF

chmod +x "$IOS_LEANC_WRAPPER" "$IOS_AR_WRAPPER"

echo "== Configure host build =="
host_cmake_args=(
  -S "$ROOT_DIR"
  -B "$HOST_LEAN_BUILD_DIR"
  -DUSE_GMP=OFF
  -DUSE_LIBUV=OFF
)
cmake "${host_cmake_args[@]}"

if [[ "$BUILD_HOST_OLEANS" != "0" ]]; then
  echo "== Build host stage1 stdlib and oleans =="
  cmake --build "$HOST_LEAN_BUILD_DIR" --target stage1-configure -j"$NPROC"
  cmake --build "$HOST_LEAN_STAGE1_BUILD_DIR" --target make_stdlib -j"$NPROC"
fi

echo "== Configure iOS runtime =="
runtime_cmake_args=(
  -S "$ROOT_DIR/src"
  -B "$LEAN_RUNTIME_BUILD_DIR"
  -G "Unix Makefiles"
  -DSTAGE=0
  -DCMAKE_SYSTEM_NAME=iOS
  -DCMAKE_OSX_SYSROOT="$IOS_SDK"
  -DCMAKE_OSX_ARCHITECTURES=arm64
  -DCMAKE_OSX_DEPLOYMENT_TARGET="$IOS_DEPLOYMENT_TARGET"
  -DCMAKE_BUILD_TYPE=Release
  -DUSE_MIMALLOC=OFF
  -DUSE_LIBUV=OFF
  -DUSE_GMP=OFF
)
cmake "${runtime_cmake_args[@]}"

echo "== Build iOS runtime libs =="
cmake --build "$LEAN_RUNTIME_BUILD_DIR" --target leanrt leancpp leanshell leaninitialize -j"$NPROC"

echo "== Build iOS stdlib libs =="
mkdir -p "$LIB_DIR"
if [[ ! -x "$IOS_LEANMAKE" ]]; then
  echo "missing generated leanmake at $IOS_LEANMAKE" >&2
  exit 1
fi
for PKG in Init Std Lean; do
  (
    cd "$ROOT_DIR/stage0/stdlib"
    IOS_SDK="$IOS_SDK" \
    IOS_TARGET="$IOS_TARGET" \
    IOS_DEPLOYMENT_TARGET="$IOS_DEPLOYMENT_TARGET" \
    LEAN_RUNTIME_INCLUDE="$LEAN_RUNTIME_BUILD_DIR/include" \
    LEAN_STAGE0_INCLUDE="$ROOT_DIR/stage0/src/include" \
    LEAN_SRC_INCLUDE="$ROOT_DIR/src/include" \
    "$IOS_LEANMAKE" \
      lib \
      -j"$NPROC" \
      LEANC="$IOS_LEANC_WRAPPER" \
      LEAN_AR="$IOS_AR_WRAPPER" \
      PKG="$PKG" \
      C_ONLY=1 \
      C_OUT=. \
      OUT="$IOS_BUILD_DIR/leanmake" \
      TEMP_OUT="$IOS_BUILD_DIR/leanmake/temp" \
      LIB_OUT="$LIB_DIR"
  )
done

echo "== Package iOS artifacts =="
IOS_PACKAGE_DIR="$BUILD_DIR/package/lean4-ios-${SDK_TAG}"
rm -rf "$IOS_PACKAGE_DIR"
mkdir -p \
  "$IOS_PACKAGE_DIR/lib" \
  "$IOS_PACKAGE_DIR/include/runtime" \
  "$IOS_PACKAGE_DIR/include/src" \
  "$IOS_PACKAGE_DIR/include/stage0"

cp "$RUNTIME_LIB" "$IOS_PACKAGE_DIR/lib/"
cp "$RUNTIME_CPP_LIB" "$IOS_PACKAGE_DIR/lib/"
cp "$RUNTIME_SHELL_LIB" "$IOS_PACKAGE_DIR/lib/"
cp "$RUNTIME_INIT_LIB" "$IOS_PACKAGE_DIR/lib/"
cp "$STDLIB_INIT_LIB" "$IOS_PACKAGE_DIR/lib/"
cp "$STDLIB_STD_LIB" "$IOS_PACKAGE_DIR/lib/"
cp "$STDLIB_LEAN_LIB" "$IOS_PACKAGE_DIR/lib/"
cp -R "$LEAN_RUNTIME_BUILD_DIR/include/." "$IOS_PACKAGE_DIR/include/runtime/"
cp -R "$ROOT_DIR/src/include/." "$IOS_PACKAGE_DIR/include/src/"
cp -R "$ROOT_DIR/stage0/src/include/." "$IOS_PACKAGE_DIR/include/stage0/"

cat >"$IOS_PACKAGE_DIR/manifest.txt" <<EOF
lean_revision=$LEAN_REV
ios_sdk=$IOS_SDK
ios_target=$IOS_TARGET
ios_deployment_target=$IOS_DEPLOYMENT_TARGET
EOF

rm -f "$IOS_ARCHIVE"
tar -C "$BUILD_DIR/package" -cf - "$(basename "$IOS_PACKAGE_DIR")" | zstd -19 -T0 -o "$IOS_ARCHIVE"
echo "Created $IOS_ARCHIVE"

if [[ "$BUILD_HOST_OLEANS" != "0" ]]; then
  echo "== Package host oleans =="
  HOST_PACKAGE_DIR="$BUILD_DIR/package/lean4-host-oleans-macos"
  rm -rf "$HOST_PACKAGE_DIR"
  mkdir -p "$HOST_PACKAGE_DIR/lib/lean"
  rsync -a --delete "$HOST_OLEAN_DIR/" "$HOST_PACKAGE_DIR/lib/lean/"
  cat >"$HOST_PACKAGE_DIR/manifest.txt" <<EOF
lean_revision=$LEAN_REV
host_platform=macos
stage=stage1
EOF
  rm -f "$HOST_ARCHIVE"
  tar -C "$BUILD_DIR/package" -cf - "$(basename "$HOST_PACKAGE_DIR")" | zstd -19 -T0 -o "$HOST_ARCHIVE"
  echo "Created $HOST_ARCHIVE"
fi
