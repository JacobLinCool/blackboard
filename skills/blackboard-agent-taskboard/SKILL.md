---
name: blackboard-agent-taskboard
description: Plan and operate blackboard taskboards for large multi-step work with explicit checkpoints, dependencies, and role ownership. Use when tasks need 30 minutes or more of focused work, staged implementation and review gates (for example security, quality, validation), durable status tracking, or controlled re-planning in single-agent or multi-agent workflows.
---

# Blackboard Agent Taskboard

## Purpose

Use `blackboard` as a structured execution board for large work that requires:

- multiple checkpoints
- quality gates
- explicit dependencies
- controlled re-planning

## Activation Cues

Activate this skill when the user asks for one or more of:

- large task decomposition into trackable steps
- staged workflow (`implement -> review -> validate -> finalize`)
- gate-based tracking (security, quality, acceptance, release)
- role-based responsibility split across people or agents
- persistent task state instead of temporary notes
- explicit dependency management and ordering
- any task that needs 30 minutes or more of focused work

Do not activate for tiny one-step work that can finish without checkpoints.

## Operating Modes

- single-agent mode: one actor plans, executes, reviews, and closes through explicit checkpoint tasks
- multi-role mode: responsibilities are split by role with least-privilege permissions

`manager/pm/engineer` is only one template, not a fixed model.

## Core Rules

- keep one project per board
- represent each major phase as a task
- represent each gate as its own checkpoint task
- make each 30-minute microtask its own task for clear ownership and tracking
- it's okay to have dozens of tasks if the work is large and complex, but avoid unnecessary microtask fragmentation
- encode ordering with dependencies, never by informal text only
- on scope change, pause execution and re-plan before resuming
- in multi-role mode, only planning roles can mutate plan content
- `--user` must be the blackboard actor identity (agent name or stable agent id), not an operating-system username

## Role Archetypes (Multi-Role Mode)

| Role Archetype    | Primary Use Case                            | Recommended Permissions      | Typical Commands                     |
| ----------------- | ------------------------------------------- | ---------------------------- | ------------------------------------ |
| board-admin       | governance, membership, permission control  | owner (implicit all)         | `board create/grant/revoke/members/delete` |
| planner           | decomposition, dependency management        | `read,create,update,delete`  | `task add/edit/delete/list/view`     |
| implementer       | implementation execution                    | `read,set_status`            | `task list/view/status`              |
| security-reviewer | security checkpoint ownership               | `read,set_status`            | `task list/view/status`              |
| quality-reviewer  | code quality checkpoint ownership           | `read,set_status`            | `task list/view/status`              |
| validator         | acceptance verification checkpoint          | `read,set_status`            | `task list/view/status`              |
| release-owner     | final go/no-go and release completion       | `read,set_status` (or planner) | `task view/status`                 |
| observer          | audit visibility                            | `read`                       | `task list/view`, `board view`       |

For any custom role set, map responsibilities first, then assign least-privilege permissions.

## Procedure

### 1) Bootstrap Board

Single-agent:

```bash
blackboard init --user <owner>
blackboard board create --user <owner> --name <board>
```

Multi-role baseline template (`manager/pm/engineer`):

```bash
BLACKBOARD_BIN=blackboard \
  ./scripts/bootstrap_project_board.sh <manager> <board> \
    <pm:read,create,update,delete> \
    <engineer:read,set_status>
```

Multi-role custom mapping:

```bash
BLACKBOARD_BIN=blackboard \
  ./scripts/bootstrap_project_board.sh <board_admin> <board> \
    <planner_user:read,create,update,delete> \
    <implementer_user:read,set_status> \
    <security_user:read,set_status> \
    <quality_user:read,set_status> \
    <observer_user:read>
```

### 2) Create Checkpoint Graph

Recommended default chain:

1. implementation
2. security check
3. quality check
4. acceptance validation
5. finalize/release

```bash
blackboard task add --user <planner> --board <board> --title "Phase 1: Implement" --description "Deliver the implementation."
blackboard task add --user <planner> --board <board> --title "Gate 1: Security Review" --description "Review and resolve security findings." --depends-on "1"
blackboard task add --user <planner> --board <board> --title "Gate 2: Quality Review" --description "Run quality checks and resolve issues." --depends-on "2"
blackboard task add --user <planner> --board <board> --title "Gate 3: Acceptance Validation" --description "Validate against acceptance criteria." --depends-on "3"
blackboard task add --user <planner> --board <board> --title "Phase 2: Finalize/Release" --description "Finalize changes and close delivery." --depends-on "4"
```

Use dependencies to enforce order.

### 3) Run Role-Specific Loops

Planner loop:

- add/edit/delete tasks
- maintain dependencies and task definitions
- re-plan when scope changes or blockers persist

Executor/reviewer loop:

- list and view assigned tasks
- move status (`pending`, `in_progress`, `completed`, `blocked`)
- report blockers through status and notes in updates

### 4) Apply Feedback Loops

Default loop for each gate:

1. run check
2. if failing, mark `blocked` or keep `in_progress` and fix
3. re-run check
4. only mark `completed` when pass criteria are met

### 5) Handle Scope Changes

1. pause active execution tasks
2. planner updates task graph and dependencies
3. board-admin updates permissions if role ownership changed
4. resume execution on revised tasks

## Machine-Readable Handoff

Prefer `--json` for inter-agent communication:

```bash
blackboard --json task list --user <actor> --board <board>
blackboard --json task view --user <actor> --board <board> --task-id <id>
blackboard --json task status --user <actor> --board <board> --task-id <id> --status completed
blackboard --json task status --user <actor> --board <board> --task-id <id> --status blocked
```

Output contract:

- success: `{"ok":true,"lines":[...]}`
- failure: `{"ok":false,"error":"..."}`

## Failure Policy

- permission denied: board-admin must resolve grants/revokes
- dependency validation error: planner must correct task graph
- missing checkpoint/gate: planner must add explicit checkpoint tasks
- unclear requirements: stop execution and request re-plan

Never bypass role boundaries or checkpoint gates as fallback behavior.

## Resources

- `scripts/bootstrap_project_board.sh`: one-shot setup for custom member-permission mappings
- `scripts/seed_project_tasks.sh`: baseline task seeding for planner-driven flows
- `references/examples.md`: scenario walkthroughs for single-agent and multi-role execution
- `references/role-use-cases.md`: role-specific responsibilities and command boundaries
