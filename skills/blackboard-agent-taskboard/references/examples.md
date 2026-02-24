# Blackboard Taskboard Scenarios

## Contents

1. Baseline multi-role bootstrap
2. Custom specialized multi-role bootstrap
3. Role-restricted execution flow
4. Scope-change re-plan flow
5. Single-agent large-task flow

## 1) Baseline Multi-Role Bootstrap

This uses the built-in baseline template (`manager/pm/engineer`).

```bash
BLACKBOARD_BIN=./target/debug/blackboard \
  ./scripts/bootstrap_project_board.sh manager project-alpha \
    pm:read,create,update,delete \
    engineer:read,set_status
```

Expected result:

- board `project-alpha` exists
- `pm` has `read,create,update,delete`
- `engineer` has `read,set_status`

## 2) Custom Specialized Multi-Role Bootstrap

Use this when responsibilities are split by specialty.

```bash
BLACKBOARD_BIN=./target/debug/blackboard \
  ./scripts/bootstrap_project_board.sh lead project-beta \
    planner:read,create,update,delete \
    implementer:read,set_status \
    security:read,set_status \
    qa:read,set_status
```

Expected result:

- planner owns plan mutations
- implementer/security/qa execute gate-specific status updates
- checkpoint chain can be modeled with dependencies

## 3) Role-Restricted Execution Flow

Planner creates and updates plan:

```bash
blackboard task add --user planner --board project-beta --title "Phase 1: Implement" --description "Implement feature."
blackboard task add --user planner --board project-beta --title "Gate 1: Security Review" --description "Review and fix findings." --depends-on "1"
blackboard task add --user planner --board project-beta --title "Gate 2: Quality Review" --description "Run quality checks." --depends-on "2"
```

Executor/reviewer updates status only:

```bash
blackboard task status --user implementer --board project-beta --task-id 1 --status completed
blackboard task status --user security --board project-beta --task-id 2 --status in_progress
blackboard task status --user security --board project-beta --task-id 2 --status blocked
```

Executor cannot mutate plan:

```bash
blackboard task add --user implementer --board project-beta --title "should fail" --description "x"
# -> forbidden: create denied
```

## 4) Scope-Change Re-Plan Flow

When requirements change:

1. pause ongoing execution updates
2. planner updates task graph
3. resume execution on revised checkpoints

```bash
blackboard task add --user planner --board project-beta --title "Gate X: Change Impact Analysis" --description "Assess new requirement impact."
blackboard task edit --user planner --board project-beta --task-id 3 --description "Revised after scope update."
```

## 5) Single-Agent Large-Task Flow

```bash
blackboard init --user owner
blackboard board create --user owner --name release-2026-q1
blackboard task add --user owner --board release-2026-q1 --title "Phase 1: Implement" --description "Complete implementation."
blackboard task add --user owner --board release-2026-q1 --title "Gate 1: Security Check" --description "Security review and fixes." --depends-on "1"
blackboard task add --user owner --board release-2026-q1 --title "Gate 2: Code Quality Check" --description "Quality checks and cleanup." --depends-on "2"
blackboard task add --user owner --board release-2026-q1 --title "Gate 3: Acceptance Validation" --description "Validate acceptance criteria." --depends-on "3"
```

Execution loop:

```bash
blackboard task list --user owner --board release-2026-q1
blackboard task status --user owner --board release-2026-q1 --task-id 1 --status in_progress
blackboard task status --user owner --board release-2026-q1 --task-id 1 --status completed
blackboard task status --user owner --board release-2026-q1 --task-id 2 --status in_progress
```
