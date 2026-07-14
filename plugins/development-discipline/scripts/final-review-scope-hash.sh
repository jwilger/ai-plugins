#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat >&2 <<'EOF'
Usage: final-review-scope-hash.sh --project-root PATH --scope base|uncommitted [--base REF] --baseline-commit OID --changed-files-from FILE
EOF
}

die() {
  printf 'final-review-scope-hash: %s\n' "$1" >&2
  exit 2
}

project_root=""
scope=""
base="origin/main"
baseline_commit=""
changed_files_from=""
changed_files=()
max_changed_files=20000
max_inventory_bytes=$((2 * 1024 * 1024))
max_git_arg_bytes=$((128 * 1024))

git_in_project() {
  command git -c core.fsmonitor=false -C "$project_root" "$@"
}

while (($# > 0)); do
  case "$1" in
    --project-root)
      (($# >= 2)) || die "--project-root requires a value"
      project_root="$2"
      shift 2
      ;;
    --scope)
      (($# >= 2)) || die "--scope requires a value"
      scope="$2"
      shift 2
      ;;
    --base)
      (($# >= 2)) || die "--base requires a value"
      base="$2"
      shift 2
      ;;
    --baseline-commit)
      (($# >= 2)) || die "--baseline-commit requires a value"
      baseline_commit="$2"
      shift 2
      ;;
    --changed-files-from)
      (($# >= 2)) || die "--changed-files-from requires a value"
      changed_files_from="$2"
      shift 2
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      usage
      die "unknown argument: $1"
      ;;
  esac
done

[[ -n "$project_root" ]] || die "--project-root must not be empty"
[[ -d "$project_root" ]] || die "project root is not a directory: $project_root"
git_in_project rev-parse --is-inside-work-tree >/dev/null 2>&1 ||
  die "project root is not inside a Git worktree: $project_root"

case "$scope" in
  base)
    [[ -n "$base" ]] || die "--base must not be empty"
    ;;
  uncommitted)
    ;;
  *)
    die "--scope must be 'base' or 'uncommitted'"
    ;;
esac
[[ "$baseline_commit" =~ ^[0-9a-f]{40}([0-9a-f]{24})?$ ]] ||
  die "--baseline-commit must be a full lowercase commit OID"

if [[ -n "$changed_files_from" ]]; then
  [[ -f "$changed_files_from" && ! -L "$changed_files_from" ]] ||
    die "changed-files inventory must be a regular file"
  inventory_bytes="$(wc -c <"$changed_files_from")"
  ((inventory_bytes <= max_inventory_bytes)) ||
    die "changed-files inventory is too large"
  while IFS= read -r -d '' path; do
    changed_files+=("$path")
    ((${#changed_files[@]} <= max_changed_files)) ||
      die "too many changed paths"
  done <"$changed_files_from"
  parsed_inventory_bytes="$({ printf '%s\0' "${changed_files[@]}"; } | wc -c)"
  ((parsed_inventory_bytes == inventory_bytes)) ||
    die "changed-files inventory must be NUL-delimited"
else
  die "--changed-files-from is required"
fi

((${#changed_files[@]} > 0)) || die "at least one changed path is required"
((${#changed_files[@]} <= max_changed_files)) || die "too many changed paths"

export LC_ALL=C
sorted_files=("${changed_files[@]}")
file_count=${#sorted_files[@]}
width=1
while ((width < file_count)); do
  scratch=()
  output_index=0
  for ((start = 0; start < file_count; start += 2 * width)); do
    left=$start
    middle=$((start + width))
    end=$((start + (2 * width)))
    ((middle < file_count)) || middle=$file_count
    ((end < file_count)) || end=$file_count
    right=$middle

    while ((left < middle || right < end)); do
      if ((right >= end)) || {
        ((left < middle)) &&
          [[ "${sorted_files[$left]}" < "${sorted_files[$right]}" || "${sorted_files[$left]}" == "${sorted_files[$right]}" ]]
      }; then
        scratch[$output_index]="${sorted_files[$left]}"
        left=$((left + 1))
      else
        scratch[$output_index]="${sorted_files[$right]}"
        right=$((right + 1))
      fi
      output_index=$((output_index + 1))
    done
  done
  sorted_files=("${scratch[@]}")
  width=$((width * 2))
done

file_modes=()
link_targets=()
gitlink_index_oids=()
gitlink_worktree_oids=()
regular_paths=()
previous=""
for ((i = 0; i < ${#sorted_files[@]}; i++)); do
  path="${sorted_files[$i]}"
  [[ -n "$path" ]] || die "changed paths must not be empty"
  [[ "$path" != /* ]] || die "changed paths must be relative to the project root: $path"
  case "/$path/" in
    */./* | */../* | *//* ) die "changed path is not normalized: $path" ;;
  esac
  if ((i > 0)) && [[ "$path" == "$previous" ]]; then
    die "changed paths must not contain duplicates: $path"
  fi
  previous="$path"

  full_path="$project_root/$path"
  if [[ -L "$full_path" ]]; then
    file_modes[$i]="120000"
    link_output="$(readlink "$full_path"; printf '.')"
    link_target="${link_output%.}"
    link_targets[$i]="${link_target%$'\n'}"
  elif [[ -f "$full_path" ]]; then
    if [[ -x "$full_path" ]]; then
      file_modes[$i]="100755"
    else
      file_modes[$i]="100644"
    fi
    regular_paths+=("$path")
  elif [[ ! -e "$full_path" ]]; then
    file_modes[$i]="deleted"
  else
    pathspec=":(literal)$path"
    pathspec_bytes=$((${#pathspec} + 16))
    ((pathspec_bytes <= max_git_arg_bytes)) || die "changed path is too large for Git"
    index_entry="$(
      git_in_project ls-files \
        --format='%(objectmode) %(objectname)' \
        -- \
        "$pathspec"
    )" || die "failed to inspect changed path mode: $path"
    if [[ "$index_entry" =~ ^160000\ ([0-9a-f]+)$ ]]; then
      file_modes[$i]="160000"
      gitlink_index_oids[$i]="${BASH_REMATCH[1]}"
      gitlink_worktree_oids[$i]="$(
        command git -c core.fsmonitor=false -C "$full_path" \
          rev-parse --verify --end-of-options 'HEAD^{commit}'
      )" || die "failed to resolve changed gitlink worktree commit: $path"
    else
      die "changed path is not a file, symlink, gitlink, or deletion: $path"
    fi
  fi
done

base_oid="$(git_in_project rev-parse --verify --end-of-options "$baseline_commit^{commit}")" ||
  die "baseline does not resolve to a commit: $baseline_commit"
[[ "$base_oid" == "$baseline_commit" ]] ||
  die "--baseline-commit must be the canonical commit OID"

index_chunk_hashes=()
worktree_chunk_hashes=()
pathspec_chunk=()
pathspec_chunk_bytes=0

hash_diff_chunk() {
  local index_chunk_hash
  local worktree_chunk_hash

  ((${#pathspec_chunk[@]} > 0)) || return 0
  index_chunk_hash="$({
    git_in_project diff \
      --cached \
      --binary \
      --full-index \
      --no-color \
      --no-ext-diff \
      --no-textconv \
      --no-renames \
      --src-prefix=a/ \
      --dst-prefix=b/ \
      "$base_oid" \
      -- \
      "${pathspec_chunk[@]}"
  } | git_in_project hash-object --stdin)" ||
    die "failed to hash base-to-index diff chunk"
  worktree_chunk_hash="$({
    git_in_project diff \
      --binary \
      --full-index \
      --no-color \
      --no-ext-diff \
      --no-textconv \
      --no-renames \
      --src-prefix=a/ \
      --dst-prefix=b/ \
      -- \
      "${pathspec_chunk[@]}"
  } | git_in_project hash-object --stdin)" ||
    die "failed to hash index-to-worktree diff chunk"
  index_chunk_hashes+=("$index_chunk_hash")
  worktree_chunk_hashes+=("$worktree_chunk_hash")
  pathspec_chunk=()
  pathspec_chunk_bytes=0
}

for path in "${sorted_files[@]}"; do
  pathspec=":(literal)$path"
  pathspec_bytes=$((${#pathspec} + 16))
  ((pathspec_bytes <= max_git_arg_bytes)) || die "changed path is too large for Git"
  if ((${#pathspec_chunk[@]} > 0 && pathspec_chunk_bytes + pathspec_bytes > max_git_arg_bytes)); then
    hash_diff_chunk
  fi
  pathspec_chunk+=("$pathspec")
  pathspec_chunk_bytes=$((pathspec_chunk_bytes + pathspec_bytes))
done
hash_diff_chunk

index_hash="$({
  printf 'final-review-diff-chunks-v1\0'
  for chunk_hash in "${index_chunk_hashes[@]}"; do
    printf 'chunk\0%s\0' "$chunk_hash"
  done
} | git_in_project hash-object --stdin)"

worktree_hash="$({
  printf 'final-review-diff-chunks-v1\0'
  for chunk_hash in "${worktree_chunk_hashes[@]}"; do
    printf 'chunk\0%s\0' "$chunk_hash"
  done
} | git_in_project hash-object --stdin)"

regular_hashes=()
regular_chunk=()
regular_chunk_bytes=0

hash_regular_chunk() {
  local regular_hash_output

  ((${#regular_chunk[@]} > 0)) || return 0
  if ! regular_hash_output="$(
    git_in_project hash-object --no-filters -- "${regular_chunk[@]}"
  )"; then
    die "failed to hash regular changed paths"
  fi
  while IFS= read -r blob_hash; do
    regular_hashes+=("$blob_hash")
  done <<<"$regular_hash_output"
  regular_chunk=()
  regular_chunk_bytes=0
}

for path in "${regular_paths[@]}"; do
  regular_path_bytes=$((${#path} + 16))
  ((regular_path_bytes <= max_git_arg_bytes)) || die "changed path is too large for Git"
  if ((${#regular_chunk[@]} > 0 && regular_chunk_bytes + regular_path_bytes > max_git_arg_bytes)); then
    hash_regular_chunk
  fi
  regular_chunk+=("$path")
  regular_chunk_bytes=$((regular_chunk_bytes + regular_path_bytes))
done
hash_regular_chunk
((${#regular_hashes[@]} == ${#regular_paths[@]})) ||
  die "Git returned an unexpected number of regular-file hashes"

manifest_hash="$({
  regular_index=0
  for ((i = 0; i < ${#sorted_files[@]}; i++)); do
    path="${sorted_files[$i]}"
    mode="${file_modes[$i]}"
    case "$mode" in
      120000)
        printf 'path\0%s\0mode\0%s\0target\0%s\0' "$path" "$mode" "${link_targets[$i]}"
        ;;
      160000)
        printf 'path\0%s\0mode\0%s\0index-oid\0%s\0worktree-oid\0%s\0' \
          "$path" \
          "$mode" \
          "${gitlink_index_oids[$i]}" \
          "${gitlink_worktree_oids[$i]}"
        ;;
      deleted)
        printf 'path\0%s\0mode\0%s\0deleted\0' "$path" "$mode"
        ;;
      *)
        printf 'path\0%s\0mode\0%s\0blob\0%s\0' \
          "$path" \
          "$mode" \
          "${regular_hashes[$regular_index]}"
        regular_index=$((regular_index + 1))
        ;;
    esac
  done
} | git_in_project hash-object --stdin)"

{
  printf 'final-review-scope-hash-v3\0'
  printf 'scope\0%s\0' "$scope"
  printf 'base\0%s\0' "$base_oid"
  printf 'index\0%s\0' "$index_hash"
  printf 'worktree\0%s\0' "$worktree_hash"
  printf 'manifest\0%s\0' "$manifest_hash"
} | git_in_project hash-object --stdin
