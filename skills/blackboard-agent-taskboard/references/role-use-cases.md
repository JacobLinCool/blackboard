# Multi-Role Use Cases

## Purpose

This reference explains how to assign responsibilities in multi-role workflows.

## 1) `board-admin`

Use case:

- project governance
- board setup and membership control

Recommended permissions:

- owner (implicit all)

Typical actions:

- create board
- grant/revoke permissions
- verify final role boundaries

## 2) `planner`

Use case:

- decompose work into phases and checkpoints
- maintain dependency graph
- re-plan when scope changes

Recommended permissions:

- `read,create,update,delete`

Typical actions:

- `task add`
- `task edit`
- `task delete`
- update `--depends-on`

## 3) `implementer`

Use case:

- execute implementation checkpoints

Recommended permissions:

- `read,set_status`

Typical actions:

- `task list`
- `task view`
- `task status`

## 4) `security-reviewer`

Use case:

- own security gate tasks
- mark blockers for unresolved findings

Recommended permissions:

- `read,set_status`

Typical actions:

- `task view` security checkpoints
- set gate status to `in_progress`, `blocked`, or `completed`

## 5) `quality-reviewer`

Use case:

- own code-quality gate tasks (lint, style, static checks)

Recommended permissions:

- `read,set_status`

Typical actions:

- `task view` quality checkpoints
- set gate status based on pass/fail outcome

## 6) `validator`

Use case:

- own acceptance validation gate
- confirm delivery matches requirements

Recommended permissions:

- `read,set_status`

Typical actions:

- validate acceptance criteria task
- mark final validation checkpoint `completed` or `blocked`

## 7) `release-owner`

Use case:

- final readiness and closeout checkpoint

Recommended permissions:

- `read,set_status` for strict mode
- add planner permissions only if release owner must edit plan

Typical actions:

- inspect release checkpoint tasks
- mark release gate status

## 8) `observer`

Use case:

- read-only visibility for stakeholders

Recommended permissions:

- `read`

Typical actions:

- `task list`
- `task view`
- `board view`

## Assignment Guidance

- Start from responsibilities, then assign permissions.
- Keep mutation rights only on planner roles unless explicitly required.
- Use separate gate tasks for security, quality, and validation ownership.
- If one role covers multiple gates, keep checkpoints separate for traceability.
