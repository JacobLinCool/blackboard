#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'EOF'
Usage:
  bootstrap_project_board.sh <board_admin> <board> <member:permissions> [member:permissions...]

Identity:
  <board_admin> and member names are blackboard actor identities (agent name/id), not OS usernames.

Permissions:
  read,create,update,delete,set_status,assign,delete_board

Examples:
  bootstrap_project_board.sh lead project-alpha \
    planner:read,create,update,delete \
    implementer:read,set_status \
    security:read,set_status \
    qa:read,set_status

  bootstrap_project_board.sh owner release-q1 observer:read

Environment:
  BLACKBOARD_BIN   Path to blackboard binary (default: blackboard)
EOF
}

if [[ $# -lt 3 ]]; then
  usage
  exit 1
fi

BOARD_ADMIN="$1"
BOARD="$2"
shift 2
BLACKBOARD_BIN="${BLACKBOARD_BIN:-blackboard}"

bb() {
  "$BLACKBOARD_BIN" "$@"
}

validate_permissions() {
  local permissions="$1"
  local permission
  IFS=',' read -r -a permission_list <<<"$permissions"
  if [[ ${#permission_list[@]} -eq 0 ]]; then
    echo "invalid empty permissions set" >&2
    exit 1
  fi
  for permission in "${permission_list[@]}"; do
    case "$permission" in
      read|create|update|delete|set_status|assign|delete_board) ;;
      *)
        echo "invalid permission: $permission" >&2
        exit 1
        ;;
    esac
  done
}

bb init --user "$BOARD_ADMIN"
bb board create --user "$BOARD_ADMIN" --name "$BOARD"

for member_spec in "$@"; do
  if [[ "$member_spec" != *:* ]]; then
    echo "invalid member spec: $member_spec (expected <member:permissions>)" >&2
    exit 1
  fi

  member="${member_spec%%:*}"
  permissions="${member_spec#*:}"

  if [[ -z "$member" || -z "$permissions" ]]; then
    echo "invalid member spec: $member_spec (member and permissions must be non-empty)" >&2
    exit 1
  fi

  validate_permissions "$permissions"

  if [[ "$member" != "$BOARD_ADMIN" ]]; then
    bb user add --user "$BOARD_ADMIN" --name "$member"
  fi

  bb board grant --user "$BOARD_ADMIN" --board "$BOARD" --target "$member" --permissions "$permissions"
done

echo "member permissions for ${BOARD}:"
bb board members --user "$BOARD_ADMIN" --board "$BOARD"
