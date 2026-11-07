#!/usr/bin/env bash
# Fast pre-push check: format, lint and test only what changed since the merge
# base with origin/main (or the ref given as $1). CI runs the full suite.
# --plan prints the commands without running them.
set -euo pipefail

plan=0
[[ "${1:-}" == --plan ]] && { plan=1; shift; }
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
cd "$repo"
[[ -f .brn-dev/env.sh ]] && source .brn-dev/env.sh
base=$(git merge-base HEAD "${1:-origin/main}")

run() {
  printf '+ %s\n' "$*"
  ((plan)) || "$@"
}

packages=() features=() workspace=0 markdown=0 shell=0 scripts=0 bench=0 budget=0
add_package() {
  local name=$1 p
  for p in ${packages[@]+"${packages[@]}"}; do [[ "$p" == "$name" ]] && return; done
  packages+=("$name")
  case "$name" in
    brn-desktop) features+=(brn-desktop/native-ui) ;;
    brn-writing-ui) features+=(brn-writing-ui/native-ui brn-writing-ui/native-test-support) ;;
  esac
}

while IFS= read -r file; do
  case "$file" in
    crates/*.md) ;;
    crates/*/*)
      dir=${file#crates/}; dir=${dir%%/*}
      [[ -f "crates/$dir/Cargo.toml" ]] && add_package "$(sed -n 's/^name *= *"\(.*\)"/\1/p' "crates/$dir/Cargo.toml" | head -1)" ;;
    Cargo.toml|Cargo.lock|rust-toolchain.toml) workspace=1 ;;
  esac
  case "$file" in
    AGENTS.md|agent/*) budget=1 ;;
  esac
  case "$file" in
    *.md) markdown=1 ;;
    *.sh) shell=1 ;;
  esac
  case "$file" in
    scripts/*) scripts=1 ;;
    benchmarks/*) bench=1 ;;
  esac
done < <({ git diff --name-only "$base"; git ls-files --others --exclude-standard; } | sort -u)

run git diff --check "$base"
if ((workspace)); then
  echo "Workspace-wide inputs changed; checking the whole workspace compiles. CI runs the full suite."
  run cargo check --workspace --all-targets --locked
fi
if ((${#packages[@]})); then
  pkg_args=()
  for p in "${packages[@]}"; do pkg_args+=(-p "$p"); done
  feature_args=()
  if ((${#features[@]})); then
    joined=$(IFS=,; echo "${features[*]}")
    feature_args=(--features "$joined")
  fi
  run cargo fmt "${pkg_args[@]}" -- --check
  run cargo clippy "${pkg_args[@]}" ${feature_args[@]+"${feature_args[@]}"} --all-targets --locked -- -D warnings
  run cargo test "${pkg_args[@]}" ${feature_args[@]+"${feature_args[@]}"} --locked
fi
((markdown)) && run python3 scripts/check-markdown-links.py
((budget)) && run python3 scripts/check-agent-context-budget.py
if ((shell)); then
  for script in scripts/*.sh; do run bash -n "$script"; done
fi
((scripts)) && run python3 -m unittest discover -s scripts/tests
((bench)) && run python3 -m unittest discover -s benchmarks/brn_agent/tests
echo "check-changed: done"
