#!/usr/bin/env bash
# Report allocated disk usage for explicitly selected local paths.
set -u

export LC_ALL=C

usage() {
  cat <<'USAGE'
Usage: report-sizes.sh [--app PATH] [--build PATH] [--model-cache PATH] [--path PATH] [PATH ...]

Reports allocated disk usage for the supplied application, Rust build,
model/cache, or other explicit paths. Options may be repeated. With no path,
prints this usage text and does not inspect the filesystem.

Allocated usage is measured with macOS du -skP. Symbolic links are not
followed, including when they occur below a selected directory. Inputs that
are lexically nested are marked as overlapping and are never summed.
USAGE
}

human_size() {
  awk -v kib="$1" 'BEGIN {
    if (kib >= 1048576) {
      printf "%.2f GiB", kib / 1048576
    } else if (kib >= 1024) {
      printf "%.2f MiB", kib / 1024
    } else {
      printf "%d KiB", kib
    }
  }'
}

without_trailing_separators() {
  local path="$1"

  if [[ -z "$path" ]]; then
    printf '%s\n' "$path"
    return
  fi

  while [[ "$path" == */ && "$path" != "/" ]]; do
    path="${path%/}"
  done
  if [[ -z "$path" ]]; then
    path="/"
  fi

  printf '%s\n' "$path"
}

absolute_path_operand() {
  local path
  local absolute

  path="$(without_trailing_separators "$1")"
  case "$path" in
    /*) absolute="$path" ;;
    *) absolute="$PWD/$path" ;;
  esac

  printf '%s\n' "$absolute"
}

lexical_overlap_path() {
  local path="$1"
  local old_ifs="$IFS"
  local component
  local normalized=''
  local components=()

  IFS='/'
  read -r -a components <<< "$path"
  IFS="$old_ifs"

  for component in "${components[@]}"; do
    case "$component" in
      ''|.) continue ;;
    esac
    if [[ -z "$normalized" ]]; then
      normalized="/$component"
    else
      normalized="$normalized/$component"
    fi
  done

  if [[ -z "$normalized" ]]; then
    normalized="/"
  fi
  printf '%s\n' "$normalized"
}

label_for_kind() {
  case "$1" in
    app) printf '%s' 'application bundle/executable' ;;
    build) printf '%s' 'Rust build directory' ;;
    model-cache) printf '%s' 'model/cache path' ;;
    path) printf '%s' 'explicit path' ;;
  esac
}

kinds=()
paths=()

add_entry() {
  if [[ -z "$2" ]]; then
    printf 'Paths must be non-empty\n' >&2
    usage >&2
    exit 2
  fi
  if [[ "$2" == *$'\n'* ]]; then
    printf 'Paths containing newline characters are unsupported\n' >&2
    usage >&2
    exit 2
  fi
  kinds[${#kinds[@]}]="$1"
  paths[${#paths[@]}]="$2"
}

while (($# > 0)); do
  case "$1" in
    --help|-h)
      usage
      exit 0
      ;;
    --app|--build|--model-cache|--path)
      option="$1"
      if (($# < 2)) || [[ -z "$2" ]]; then
        printf '%s requires a non-empty path\n' "$option" >&2
        usage >&2
        exit 2
      fi
      case "$option" in
        --app) add_entry app "$2" ;;
        --build) add_entry build "$2" ;;
        --model-cache) add_entry model-cache "$2" ;;
        --path) add_entry path "$2" ;;
      esac
      shift 2
      ;;
    --)
      shift
      while (($# > 0)); do
        add_entry path "$1"
        shift
      done
      ;;
    -*)
      printf 'Unknown option: %s\n' "$1" >&2
      usage >&2
      exit 2
      ;;
    *)
      add_entry path "$1"
      shift
      ;;
  esac
done

if ((${#paths[@]} == 0)); then
  usage
  exit 0
fi

operands=()
overlap_paths=()
for path in "${paths[@]}"; do
  operand="$(absolute_path_operand "$path")"
  operands[${#operands[@]}]="$operand"
  overlap_paths[${#overlap_paths[@]}]="$(lexical_overlap_path "$operand")"
done

printf 'Measurement policy: allocated usage from du -skP; symbolic links are not followed.\n'
printf 'Overlapping entries are reported separately and are not summed; allocated usage is not download size.\n'

failures=0
for ((index = 0; index < ${#paths[@]}; index++)); do
  path="${paths[index]}"
  kind="${kinds[index]}"
  label="$(label_for_kind "$kind")"
  operand="${operands[index]}"
  overlap_path="${overlap_paths[index]}"
  overlap=''
  symlink_note=''

  for ((other = 0; other < ${#paths[@]}; other++)); do
    if ((other == index)); then
      continue
    fi
    other_overlap_path="${overlap_paths[other]}"
    if [[ "$overlap_path" == "$other_overlap_path" || "$overlap_path" == "$other_overlap_path"/* || "$other_overlap_path" == "$overlap_path"/* ]]; then
      overlap="; overlap with ${paths[other]}"
      break
    fi
  done

  if [[ -L "$operand" ]]; then
    symlink_note='; symlink not followed'
  fi

  if [[ ! -e "$operand" && ! -L "$operand" ]]; then
    printf '%s | %s | unavailable: missing%s\n' "$label" "$path" "$overlap"
    failures=1
    continue
  fi

  du_output="$(du -skP "$operand" 2>&1)"
  du_exit=$?
  if ((du_exit != 0)) || [[ ! "$du_output" =~ ^([0-9]+)[[:space:]] ]]; then
    printf '%s | %s | unavailable: permission or measurement failure%s%s\n' "$label" "$path" "$overlap" "$symlink_note"
    if [[ -n "$du_output" ]]; then
      printf '%s\n' "$du_output" >&2
    fi
    failures=1
    continue
  fi

  kib="${BASH_REMATCH[1]}"
  printf '%s | %s | allocated=%s (%s)%s%s\n' \
    "$label" "$path" "$kib KiB" "$(human_size "$kib")" "$overlap" "$symlink_note"
done

if ((failures != 0)); then
  printf 'One or more paths could not be measured; no aggregate total was computed.\n' >&2
  exit 1
fi
