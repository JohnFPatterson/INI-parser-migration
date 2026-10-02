#!/usr/bin/env bash
# Dispatch to the per-profile oracle binary based on fixture path or --profile.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PROFILE=""
ARGS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --profile)
      PROFILE="${2:-}"
      shift 2
      ;;
    --sections)
      ARGS+=(--sections "${2:-}")
      shift 2
      ;;
    *)
      FIXTURE="$1"
      ARGS+=("$1")
      shift
      ;;
  esac
done

if [[ -z "${PROFILE}" ]]; then
  if [[ -z "${FIXTURE:-}" ]]; then
    echo "usage: oracle [--profile NAME] [--sections parse] <fixture>" >&2
    exit 2
  fi
  # Expect tests/parity/<profile>/...
  if [[ "$FIXTURE" =~ tests/parity/([^/]+)/ ]]; then
    PROFILE="${BASH_REMATCH[1]}"
  else
    echo "cannot infer profile from fixture path: $FIXTURE" >&2
    exit 2
  fi
fi

BIN="$ROOT/build/oracle-$PROFILE"
if [[ ! -x "$BIN" ]]; then
  echo "missing oracle binary: $BIN (run make oracles)" >&2
  exit 1
fi
exec "$BIN" "${ARGS[@]}"
