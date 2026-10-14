#!/usr/bin/env bash
# Builds the SwiftUI app: dist/Toolbox.app and dist/Toolbox-<version>.dmg.
#
# The Rust core (crates/toolbox-ffi) is built as a static library into macos/Rust/lib, its
# Swift bindings are generated into macos/Sources, and then the Swift package is built.
#
# Usage: scripts/build-macos.sh [--native] [--no-dmg] [--dev]
#   --native   Build only for this Mac's CPU. By default the app is universal (Apple silicon + Intel).
#   --no-dmg   Skip creating the disk image.
#   --dev      Quick local build: implies --native and --no-dmg, builds Swift in debug, then
#              opens the app.
#
# Optional environment variables:
#   BUNDLE_ID       Reverse-DNS app identifier. Set it to a domain you own before distributing.
#   SIGN_IDENTITY   e.g. "Developer ID Application: Your Name (TEAMID)". Without it the app is
#                   ad-hoc signed: it runs on this Mac, but other Macs show a Gatekeeper warning.
#   NOTARY_PROFILE  Keychain profile created with `xcrun notarytool store-credentials`. When set
#                   (with SIGN_IDENTITY), the disk image is notarized by Apple and stapled.
set -euo pipefail

APP_NAME="Toolbox"
BIN_NAME="Toolbox"
BUNDLE_ID="${BUNDLE_ID:-com.example.toolbox}"
MIN_MACOS="14.0"

universal=true
make_dmg=true
dev=false
for arg in "$@"; do
    case "$arg" in
        --native) universal=false ;;
        --no-dmg) make_dmg=false ;;
        --dev) dev=true universal=false make_dmg=false ;;
        -h | --help)
            awk 'NR > 1 && /^#/ { sub(/^# ?/, ""); print; next } NR > 1 { exit }' "$0"
            exit 0
            ;;
        *)
            echo "unknown option: $arg (see --help)" >&2
            exit 2
            ;;
    esac
done

if [[ "$(uname)" != "Darwin" ]]; then
    echo "This script builds macOS releases and must run on macOS." >&2
    exit 1
fi
if ! command -v cargo > /dev/null && [[ -f "$HOME/.cargo/env" ]]; then
    source "$HOME/.cargo/env"
fi
if [[ -n "${NOTARY_PROFILE:-}" && -z "${SIGN_IDENTITY:-}" ]]; then
    echo "NOTARY_PROFILE needs SIGN_IDENTITY: Apple only notarizes Developer ID signed apps." >&2
    exit 1
fi

cd "$(dirname "$0")/.."
VERSION="$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' Cargo.toml)"
DIST="dist"
APP="$DIST/$APP_NAME.app"
ICON="assets/icon/Toolbox.icns"
step() { printf '\n==> %s\n' "$*"; }

if [[ ! -f "$ICON" ]]; then
    step "Rendering the app icon"
    cargo xtask icon
fi

# ---- Build -------------------------------------------------------------------------------
if $universal; then
    targets=(aarch64-apple-darwin x86_64-apple-darwin)
else
    targets=("$(rustc -vV | sed -n 's/^host: //p')")
fi
installed="$(rustup target list --installed)"
for target in "${targets[@]}"; do
    if ! grep -qx "$target" <<< "$installed"; then
        step "Installing the Rust standard library for $target"
        rustup target add "$target"
    fi
done

RUST_LIB="macos/Rust/lib"
libs=()
for target in "${targets[@]}"; do
    step "Building the Rust core for $target"
    MACOSX_DEPLOYMENT_TARGET="$MIN_MACOS" cargo build --release --locked -p toolbox-ffi --target "$target"
    libs+=("target/$target/release/libtoolbox_ffi.a")
done
mkdir -p "$RUST_LIB"
lipo -create -output "$RUST_LIB/libtoolbox_ffi.a" "${libs[@]}"

step "Generating Swift bindings"
generated="target/uniffi-swift"
rm -rf "$generated"
cargo run --release --locked -q -p uniffi-bindgen -- generate --library "${libs[0]}" \
    --language swift --out-dir "$generated"
# Replace the checked-out copies only when they changed, so Swift doesn't rebuild needlessly.
install_if_changed() {
    mkdir -p "$(dirname "$2")"
    cmp -s "$1" "$2" || cp "$1" "$2"
}
install_if_changed "$generated/toolbox_ffi.swift" macos/Sources/ToolboxBindings/toolbox_ffi.swift
install_if_changed "$generated/toolbox_ffiFFI.h" macos/Sources/toolbox_ffiFFI/include/toolbox_ffiFFI.h
install_if_changed "$generated/toolbox_ffiFFI.modulemap" macos/Sources/toolbox_ffiFFI/include/module.modulemap

if $dev; then configuration=debug; else configuration=release; fi
archs=()
for target in "${targets[@]}"; do
    arch="${target%%-*}"
    archs+=(--arch "${arch/aarch64/arm64}")
done
step "Building $APP_NAME $VERSION for ${targets[*]} ($configuration)"
swift_build=(swift build --package-path macos -c "$configuration" "${archs[@]}")
"${swift_build[@]}"
binary="$("${swift_build[@]}" --show-bin-path)/$BIN_NAME"

# ---- Bundle ------------------------------------------------------------------------------
step "Assembling $APP"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$binary" "$APP/Contents/MacOS/$BIN_NAME"
cp "$ICON" "$APP/Contents/Resources/$APP_NAME.icns"
cat > "$APP/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>     <string>en</string>
    <key>CFBundleDisplayName</key>           <string>$APP_NAME</string>
    <key>CFBundleDocumentTypes</key>
    <array>
        <dict>
            <key>CFBundleTypeName</key>      <string>Image</string>
            <key>CFBundleTypeRole</key>      <string>Viewer</string>
            <key>LSHandlerRank</key>         <string>Alternate</string>
            <key>LSItemContentTypes</key>
            <array>
                <string>public.image</string>
                <string>public.svg-image</string>
            </array>
        </dict>
    </array>
    <key>CFBundleExecutable</key>            <string>$BIN_NAME</string>
    <key>CFBundleIconFile</key>              <string>$APP_NAME</string>
    <key>CFBundleIdentifier</key>            <string>$BUNDLE_ID</string>
    <key>CFBundleInfoDictionaryVersion</key> <string>6.0</string>
    <key>CFBundleName</key>                  <string>$APP_NAME</string>
    <key>CFBundlePackageType</key>           <string>APPL</string>
    <key>CFBundleShortVersionString</key>    <string>$VERSION</string>
    <key>CFBundleVersion</key>               <string>$VERSION</string>
    <key>LSApplicationCategoryType</key>     <string>public.app-category.developer-tools</string>
    <key>LSMinimumSystemVersion</key>        <string>$MIN_MACOS</string>
    <key>NSHighResolutionCapable</key>       <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key> <true/>
</dict>
</plist>
EOF
plutil -lint -s "$APP/Contents/Info.plist"

# ---- Sign --------------------------------------------------------------------------------
if [[ -n "${SIGN_IDENTITY:-}" ]]; then
    step "Signing with \"$SIGN_IDENTITY\""
    codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" "$APP"
else
    step "Ad-hoc signing (set SIGN_IDENTITY to sign for distribution)"
    codesign --force --sign - "$APP"
fi
codesign --verify --strict "$APP"

# ---- Disk image --------------------------------------------------------------------------
DMG="$DIST/$APP_NAME-$VERSION.dmg"
if $make_dmg; then
    step "Creating $DMG"
    staging="$(mktemp -d)"
    trap 'rm -rf "$staging"' EXIT
    cp -R "$APP" "$staging/"
    ln -s /Applications "$staging/Applications"
    rm -f "$DMG"
    hdiutil create -quiet -volname "$APP_NAME" -srcfolder "$staging" -fs HFS+ -format UDZO "$DMG"
    if [[ -n "${SIGN_IDENTITY:-}" ]]; then
        codesign --force --timestamp --sign "$SIGN_IDENTITY" "$DMG"
    fi
    if [[ -n "${NOTARY_PROFILE:-}" ]]; then
        step "Notarizing $DMG (this can take a few minutes)"
        xcrun notarytool submit "$DMG" --keychain-profile "$NOTARY_PROFILE" --wait
        xcrun stapler staple "$DMG"
    fi
fi

# ---- Summary -----------------------------------------------------------------------------
step "Done"
echo "  App:           $APP ($(du -sh "$APP" | cut -f1), $(lipo -archs "$APP/Contents/MacOS/$BIN_NAME"))"
if $make_dmg; then
    echo "  Disk image:    $DMG ($(du -h "$DMG" | cut -f1))"
fi
echo "  Bundle ID:     $BUNDLE_ID"
echo "  Minimum macOS: $MIN_MACOS"

if $dev; then
    # Quit a running copy first so `open` launches the new build.
    running="$PWD/$APP/Contents/MacOS/$BIN_NAME"
    osascript -e "quit app \"$PWD/$APP\"" 2> /dev/null || true
    while pgrep -qf "$running"; do sleep 0.2; done
    open "$APP"
fi
