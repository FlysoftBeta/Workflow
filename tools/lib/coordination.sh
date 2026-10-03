#!/usr/bin/env bash
# Shared resources belong to the primary checkout, including when a task uses a linked worktree.
workflow_primary_root() {
  local checkout=$1 primary
  primary=$(git -C "$checkout" worktree list --porcelain 2>/dev/null | sed -n 's/^worktree //p' | head -1) || true
  primary=${primary:-$checkout}
  if [[ -n ${WORKFLOW_COORDINATION_ROOT:-} && ${WORKFLOW_COORDINATION_ROOT} != "$primary" ]]; then
    printf 'WORKFLOW_COORDINATION_ROOT must name the primary checkout: %s\n' "$primary" >&2
    return 2
  fi
  printf '%s\n' "$primary"
}
