#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'EOF'
Usage:
  seed_project_tasks.sh <planner> <board> [prefix]

Environment:
  BLACKBOARD_BIN   Path to blackboard binary (default: blackboard)
EOF
}

if [[ $# -lt 2 || $# -gt 3 ]]; then
  usage
  exit 1
fi

PLANNER="$1"
BOARD="$2"
PREFIX="${3:-Project}"
BLACKBOARD_BIN="${BLACKBOARD_BIN:-blackboard}"

bb() {
  "$BLACKBOARD_BIN" "$@"
}

bb task add --user "$PLANNER" --board "$BOARD" --title "${PREFIX}: Analyze Scope" --description "Clarify requirements, assumptions, and constraints."
bb task add --user "$PLANNER" --board "$BOARD" --title "${PREFIX}: Build Implementation Plan" --description "Break down tasks and dependencies for execution."
bb task add --user "$PLANNER" --board "$BOARD" --title "${PREFIX}: Execute and Validate" --description "Complete implementation and verify acceptance criteria."

echo "seeded tasks for ${BOARD}:"
bb task list --user "$PLANNER" --board "$BOARD"
