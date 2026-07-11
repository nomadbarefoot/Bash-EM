#!/usr/bin/env bash
# Rename legacy project name to Bash-EM.
#
# Usage:
#   ./scripts/rename-project.sh --dry-run   # preview changes
#   ./scripts/rename-project.sh             # apply changes
#
# Replaces text in source/docs, including HTML span-split branding, and renames paths.
# Includes this script itself. Skips .git/, target/, and common build dirs.

set -euo pipefail

# Built at runtime so this script survives its own text replacement pass.
OLD_NAME="$(printf 'bash-%s' 'm')"
NEW_NAME='Bash-EM'
OLD_BRAND="$(printf 'bash\xe2\x80\x94m')" # em-dash branding variant (U+2014)
NEW_BRAND='Bash-EM'

DRY_RUN=false
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dry-run|-n) DRY_RUN=true ;;
        -h|--help)
            sed -n '2,9p' "$0"
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
    shift
done

# Directories to skip during walk
SKIP_DIRS=(
    .git
    target
    node_modules
    .cargo
)

should_skip_dir() {
    local base
    base="$(basename "$1")"
    for skip in "${SKIP_DIRS[@]}"; do
        [[ "$base" == "$skip" ]] && return 0
    done
    return 1
}

is_text_file() {
    local file="$1"
    [[ -f "$file" ]] || return 1
    [[ -L "$file" ]] && return 1

    # Skip obvious binaries by extension
    case "$file" in
        *.png|*.jpg|*.jpeg|*.gif|*.ico|*.woff|*.woff2|*.ttf|*.eot|*.pdf|*.zip|*.gz|*.tar|*.rlib|*.dylib|*.so|*.a|*.o)
            return 1
            ;;
    esac

    # Heuristic: grep -I skips binary files on BSD/GNU grep
    if ! LC_ALL=C grep -Iq . "$file" 2>/dev/null; then
        return 1
    fi
    return 0
}

contains_old_name() {
    python3 - "$1" <<'PY'
import re, sys

path = sys.argv[1]
text = open(path, encoding="utf-8", errors="replace").read()
old_hyphen = "bash" + "-" + "m"
old_em = "bash" + "\u2014" + "m"
if old_hyphen in text or old_em in text:
    raise SystemExit(0)
if re.search(r"bash<span[^>]*>\u2014</span>m", text):
    raise SystemExit(0)
raise SystemExit(1)
PY
}

show_matches() {
    local file="$1"
    grep -n -F "$OLD_NAME" "$file" 2>/dev/null || true
    grep -n -F "$OLD_BRAND" "$file" 2>/dev/null || true
    python3 - "$file" <<'PY'
import re, sys

path = sys.argv[1]
for i, line in enumerate(open(path, encoding="utf-8", errors="replace"), 1):
    if re.search(r"bash<span[^>]*>\u2014</span>m", line):
        print(f"{i}:{line.rstrip()}")
PY
}

replace_in_file() {
    local file="$1"
    if ! contains_old_name "$file"; then
        return 0
    fi

    if $DRY_RUN; then
        echo "[dry-run] would edit: ${file#$ROOT/}"
        show_matches "$file"
        return 0
    fi

    local tmp
    tmp="$(mktemp)"
    python3 - "$file" "$tmp" <<'PY'
import re, sys

src, dst = sys.argv[1], sys.argv[2]
text = open(src, encoding="utf-8").read()
old_hyphen = "bash" + "-" + "m"
old_em = "bash" + "\u2014" + "m"
new_name = "Bash-EM"
text = re.sub(r"bash<span([^>]*)>\u2014</span>m", r"Bash<span\1>-</span>EM", text)
text = text.replace(old_em, new_name)
text = text.replace(old_hyphen, new_name)
open(dst, "w", encoding="utf-8").write(text)
PY
    mv "$tmp" "$file"
    echo "edited: ${file#$ROOT/}"
}

# Collect files depth-first, skipping excluded dirs
mapfile -t ALL_FILES < <(
    find "$ROOT" -type f -print 2>/dev/null | while read -r f; do
        skip=false
        rel="${f#$ROOT/}"
        IFS='/' read -ra parts <<<"$rel"
        for part in "${parts[@]}"; do
            if should_skip_dir "$part"; then
                skip=true
                break
            fi
        done
        if [[ "$skip" == true ]]; then
            continue
        fi
        echo "$f"
    done
)

MODE='apply'
$DRY_RUN && MODE='dry-run'

echo "=== ${OLD_NAME} → ${NEW_NAME} (${MODE}) ==="
echo "root: $ROOT"
echo

# 1. Text replacements
edited=0
for file in "${ALL_FILES[@]}"; do
    [[ "$file" == "$0" ]] && continue
    if is_text_file "$file" && contains_old_name "$file"; then
        replace_in_file "$file"
        ((edited++)) || true
    fi
done

# 2. Path renames (deepest first so parent dirs rename cleanly)
mapfile -t RENAME_CANDIDATES < <(
    printf '%s\n' "${ALL_FILES[@]}" | grep -F "$OLD_NAME" | awk '{ print length, $0 }' | sort -rn | cut -d' ' -f2-
)

renamed=0
for path in "${RENAME_CANDIDATES[@]}"; do
  [[ -e "$path" ]] || continue
  new_path="${path//$OLD_NAME/$NEW_NAME}"
  [[ "$path" == "$new_path" ]] && continue

  if $DRY_RUN; then
    echo "[dry-run] would rename: ${path#$ROOT/} → ${new_path#$ROOT/}"
  else
    mkdir -p "$(dirname "$new_path")"
    mv "$path" "$new_path"
    echo "renamed: ${path#$ROOT/} → ${new_path#$ROOT/}"
  fi
  ((renamed++)) || true
done

echo
echo "done: $edited file(s) edited, $renamed path(s) renamed"
if $DRY_RUN; then
    echo "Re-run without --dry-run to apply."
fi
