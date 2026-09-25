#!/usr/bin/env bash
# Build AniiSignal and stage it into the Aniimo install.
#
# The mod is a MelonLoader assembly, so it does not build with cargo. This
# script is the bridge: fetch the MelonLoader reference assemblies, build the
# DLL, and drop it plus a starter config into the game's Mods directory.
#
#   tools/build-mod.sh                 build + stage
#   tools/build-mod.sh --no-stage      build only
#   GAME_DIR=/path/to/Aniimo tools/build-mod.sh
#
# Serviimo owns the config: once the server runs it rewrites AniiSignal.json
# next to the staged DLL with its real bind address and gate public key. The
# file written here is only a development default.

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
proj_dir="$here/../AniiSignal"
lib_dir="$proj_dir/lib"
out_dir="$proj_dir/build"

GAME_DIR="${GAME_DIR:-/mnt/gamingex/Steam/steamapps/common/Aniimo}"
MELONLOADER_VERSION="${MELONLOADER_VERSION:-0.6.6}"
stage=1
[[ "${1:-}" == "--no-stage" ]] && stage=0

die() { echo "build-mod: $*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

have dotnet || die "dotnet not found; install the .NET SDK (6+ targeting support)"

# MelonLoader reference assemblies. Fetched once into lib/, which is
# gitignored - the assemblies are third-party binaries, not our source.
if [[ ! -f "$lib_dir/MelonLoader.dll" || ! -f "$lib_dir/0Harmony.dll" ]]; then
  echo "build-mod: fetching MelonLoader $MELONLOADER_VERSION reference assemblies"
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT
  url="https://github.com/LavaGang/MelonLoader/releases/download/v${MELONLOADER_VERSION}/MelonLoader.Linux.x64.zip"
  have curl || die "curl not found"
  curl -sSL -o "$tmp/ml.zip" "$url" || die "download failed: $url"
  unzip -oq "$tmp/ml.zip" -d "$tmp/x" || die "unzip failed"
  src="$tmp/x/MelonLoader/net6"
  [[ -d "$src" ]] || die "unexpected archive layout; no MelonLoader/net6"
  mkdir -p "$lib_dir"
  cp "$src/MelonLoader.dll" "$src/0Harmony.dll" "$lib_dir/"
  echo "build-mod: staged reference assemblies into $lib_dir"
fi

echo "build-mod: compiling AniiSignal"
dotnet build "$proj_dir/AniiSignal.csproj" \
  -c Release \
  -o "$out_dir" \
  -p:MelonLibDir="$lib_dir" \
  --nologo \
  -v quiet

dll="$out_dir/AniiSignal.dll"
[[ -f "$dll" ]] || die "build produced no AniiSignal.dll"

# The redirect rewrite is the one piece verifiable without the game, and a
# silent failure there sends the client to FunPlus while the log looks fine.
# MelonLoader bundles its own net6 runtime in-game; on a dev box that may only
# have a newer runtime, hence the roll-forward.
echo "build-mod: running self-check"
if ! DOTNET_ROLL_FORWARD=Major dotnet "$dll" --selfcheck; then
  die "self-check failed; not staging"
fi

echo "build-mod: built $(basename "$dll") ($(stat -c%s "$dll") bytes)"

if [[ $stage -eq 0 ]]; then
  echo "build-mod: --no-stage, leaving artifacts in $out_dir"
  exit 0
fi

[[ -d "$GAME_DIR" ]] || die "game dir not found: $GAME_DIR (set GAME_DIR, or pass --no-stage)"

mods_dir="$GAME_DIR/Mods"
mkdir -p "$mods_dir"

# MelonLoader itself has to be installed for the game to load any mod. The
# game is IL2CPP, so it needs the net6 build; we do not install it here
# because that writes into the Steam install and should be a deliberate act.
if [[ ! -f "$GAME_DIR/MelonLoader/net6/MelonLoader.dll" && ! -f "$GAME_DIR/Mods/MelonLoader.dll" ]]; then
  cat >&2 <<EOF
build-mod: WARNING - MelonLoader does not look installed in $GAME_DIR
build-mod:   the DLL is staged, but the game will not load it until
build-mod:   MelonLoader $MELONLOADER_VERSION is installed (net6 build, IL2CPP).
EOF
fi

cp "$dll" "$mods_dir/AniiSignal.dll"
echo "build-mod: staged -> $mods_dir/AniiSignal.dll"

# Development default only; the server overwrites this once it runs.
cfg="$mods_dir/AniiSignal.json"
if [[ ! -f "$cfg" ]]; then
  cat > "$cfg" <<'EOF'
{
  "CdnBaseUrl": "http://127.0.0.1:8080",
  "OfficeBaseUrl": "http://127.0.0.1:8081",
  "GatePublicKeyPem": "",
  "InjectGateKey": true,
  "RedirectEndpoints": true,
  "StubAntiCheat": true,
  "BlockOutboundTelemetry": false
}
EOF
  echo "build-mod: wrote development default -> $cfg"
else
  echo "build-mod: left existing config alone -> $cfg"
fi

echo "build-mod: done"
