#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT_DIR/target}"
PROFILE_DIR="$TARGET_DIR/release"
APP_DIR="$PROFILE_DIR/DeskHud.app"
DMG_PATH="$PROFILE_DIR/DeskHud-macos.dmg"
BINARY_PATH="$PROFILE_DIR/deskhud-egui"
PLIST_TEMPLATE="$ROOT_DIR/packaging/macos/Info.plist"
ICON_PATH="$ROOT_DIR/assets/icon.icns"
FONT_SOURCE="$PROFILE_DIR/fonts"
SKIP_BUILD=false

usage() {
    echo "Usage: $0 [--skip-build]"
}

for arg in "$@"; do
    case "$arg" in
        --skip-build) SKIP_BUILD=true ;;
        -h|--help) usage; exit 0 ;;
        *) echo "Unknown argument: $arg" >&2; usage >&2; exit 2 ;;
    esac
done

if [[ "$SKIP_BUILD" != true ]]; then
    cargo build --manifest-path "$ROOT_DIR/Cargo.toml" \
        -p deskhud-egui --release
fi

for required in "$BINARY_PATH" "$PLIST_TEMPLATE" "$ICON_PATH"; do
    if [[ ! -e "$required" ]]; then
        echo "Required file not found: $required" >&2
        exit 1
    fi
done

if [[ ! -d "$FONT_SOURCE" ]]; then
    FONT_SOURCE="$ROOT_DIR/assets/fonts"
fi
if [[ ! -d "$FONT_SOURCE" ]]; then
    echo "Bundled font directory not found: $FONT_SOURCE" >&2
    exit 1
fi

PACKAGE_ID="$(cargo pkgid --manifest-path "$ROOT_DIR/Cargo.toml" -p deskhud-egui)"
PACKAGE_VERSION="${PACKAGE_ID##*#}"
PACKAGE_VERSION="${PACKAGE_VERSION##*@}"
VERSION="${DESKHUD_VERSION:-$PACKAGE_VERSION}"

rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS" "$APP_DIR/Contents/Resources"

cp "$BINARY_PATH" "$APP_DIR/Contents/MacOS/deskhud-egui"
cp "$ICON_PATH" "$APP_DIR/Contents/Resources/icon.icns"
sed "s|@VERSION@|$VERSION|g" "$PLIST_TEMPLATE" \
    > "$APP_DIR/Contents/Info.plist"

mkdir -p "$APP_DIR/Contents/Resources/fonts"
cp -R "$FONT_SOURCE/." "$APP_DIR/Contents/Resources/fonts/"

if ! command -v hdiutil >/dev/null 2>&1; then
    echo "hdiutil is required to create a macOS DMG" >&2
    exit 1
fi

STAGING_DIR="$(mktemp -d "${TMPDIR:-/tmp}/deskhud-dmg.XXXXXX")"
RW_DMG_PATH="${TMPDIR:-/tmp}/DeskHud-rw.$$.dmg"
MOUNT_POINT=""
SETFILE_PATH="$(xcrun --find SetFile 2>/dev/null || true)"

cleanup() {
    if [[ -n "$MOUNT_POINT" ]]; then
        hdiutil detach "$MOUNT_POINT" -quiet || true
    fi
    rm -f "$RW_DMG_PATH"
    rm -rf "$STAGING_DIR"
}
trap cleanup EXIT

mkdir -p "$STAGING_DIR/.background"
cp -R "$APP_DIR" "$STAGING_DIR/DeskHud.app"
cp "$ROOT_DIR/packaging/macos/dmg-background.png" \
    "$STAGING_DIR/.background/dmg-background.png"
ln -s /Applications "$STAGING_DIR/Applications"

if [[ -z "$SETFILE_PATH" ]]; then
    echo "SetFile is required to set the DMG volume icon; install Xcode Command Line Tools" >&2
    exit 1
fi
"$SETFILE_PATH" -a C "$STAGING_DIR"

hdiutil create \
    -volname "DeskHud" \
    -fs HFS+ \
    -srcfolder "$STAGING_DIR" \
    -ov \
    -format UDRW \
    "$RW_DMG_PATH"

MOUNT_POINT="$(hdiutil attach "$RW_DMG_PATH" -nobrowse | \
    awk '/\/Volumes\// { print substr($0, index($0, "/Volumes/")); exit }')"
if [[ -z "$MOUNT_POINT" || ! -d "$MOUNT_POINT" ]]; then
    echo "Could not find the mounted DeskHud DMG volume" >&2
    exit 1
fi

mkdir -p "$MOUNT_POINT/.background"
cp "$ROOT_DIR/packaging/macos/dmg-background.png" \
    "$MOUNT_POINT/.background/dmg-background.png"
cp "$ICON_PATH" "$MOUNT_POINT/VolumeIcon.icns"
"$SETFILE_PATH" -c icnC "$MOUNT_POINT/VolumeIcon.icns"
mv "$MOUNT_POINT/VolumeIcon.icns" "$MOUNT_POINT/.VolumeIcon.icns"
chflags hidden "$MOUNT_POINT/.VolumeIcon.icns"

test -f "$MOUNT_POINT/.VolumeIcon.icns" || {
    echo "Volume icon was not copied into the mounted DMG" >&2
    exit 1
}
"$SETFILE_PATH" -a C "$MOUNT_POINT"

if ! osascript - "$MOUNT_POINT" <<'APPLESCRIPT'
on run argv
    set mountPath to item 1 of argv
    tell application "Finder"
        tell disk "DeskHud"
            open
            delay 1
            set current view of container window to icon view
            set toolbar visible of container window to false
            set statusbar visible of container window to false
            set bounds of container window to {100, 100, 820, 620}
            set viewOptions to the icon view options of container window
            set arrangement of viewOptions to not arranged
            set icon size of viewOptions to 128
            set background picture of viewOptions to file ".background:dmg-background.png"
            set position of item "DeskHud.app" of container window to {220, 300}
            set position of item "Applications" of container window to {600, 300}
            update without registering applications
            close
        end tell
    end tell
end run
APPLESCRIPT
then
    echo "Warning: Finder layout could not be applied; the DMG is still usable" >&2
fi

"$SETFILE_PATH" -a C "$MOUNT_POINT"
sync
hdiutil detach "$MOUNT_POINT" -quiet
MOUNT_POINT=""
hdiutil convert "$RW_DMG_PATH" \
    -format UDZO \
    -imagekey zlib-level=9 \
    -ov \
    -o "$DMG_PATH"

test -f "$APP_DIR/Contents/Resources/icon.icns" || {
    echo "App icon was not copied into the app bundle" >&2
    exit 1
}
test -f "$APP_DIR/Contents/Info.plist" || {
    echo "Info.plist was not generated" >&2
    exit 1
}
test -n "$(find "$APP_DIR/Contents/Resources/fonts" -type f -print -quit)" || {
    echo "No fonts were copied into the app bundle" >&2
    exit 1
}
test -f "$DMG_PATH" || {
    echo "DMG was not created: $DMG_PATH" >&2
    exit 1
}

echo "Created: $APP_DIR"
echo "Created: $DMG_PATH"
