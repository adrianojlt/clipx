# Deep code review for a React + Rust + Tauri project

You are acting as a strict staff engineer doing a deep review of this codebase.

I know very little React and almost no Rust.
Your job is to review the project, explain problems in simple language, and help me understand what is wrong structurally before we change code.

## Rules

- Do not start editing code yet.
- First understand the architecture.
- Explain things in simple language.
- When you report a problem, always include:
  - severity: critical / high / medium / low
  - file path
  - why it is a problem
  - how to fix it
  - whether it belongs to React, Rust, Tauri boundary, tests, tooling, or project structure
- Prefer small safe refactors over rewrites.
- Distinguish "style issue" from "real engineering risk".
- Flag dead code, duplication, tight coupling, poor naming, hidden complexity, unclear ownership, and low testability.
- If something is acceptable, say so clearly.

## Phase 1 — Map the project

First, inspect the repository and produce an architecture map.

Identify:
- frontend entry points
- routing structure
- major React pages/components
- state management approach
- service/api layers
- Tauri commands, invokes, events, plugins
- Rust entry points, modules, services, persistence, async tasks
- shared types/contracts between frontend and backend
- current test/lint/build setup

Output:
- a short architecture summary
- a list of what is well organized
- a list of what is poorly organized
- the main dependency flow: React -> Tauri -> Rust

## Phase 2 — Review the React ↔ Tauri boundary first

Review the boundary before reviewing React or Rust in isolation.

Look for:
- direct `invoke` calls inside UI components
- missing typed wrappers
- duplicated command names
- weak validation at the boundary
- weak error mapping
- inconsistent serialization/deserialization
- leaking backend details into frontend
- confusing async flow
- missing adapter/service layers

## Phase 3 — Review the React code

Review the frontend for:
- folder structure and separation of concerns
- component responsibilities
- component size
- prop drilling
- state misuse
- `useEffect` misuse
- duplicated logic
- poor naming
- weak loading/error/empty states
- weak testability
- styling inconsistency
- dead code
- coupling between UI and backend calls

For each issue, suggest the smallest practical refactor.

## Phase 4 — Review the Rust code

Review the Rust backend for:
- module organization
- command handler design
- error types and propagation
- `Result` handling
- `unwrap` / `expect` misuse
- ownership/cloning smells
- async/concurrency risks
- filesystem/process/security risks
- logging quality
- data model design
- testability
- code readability for a beginner

When explaining Rust issues, explain them as if teaching a backend engineer who is new to Rust.

## Phase 5 — Review tests and tooling

Check:
- formatting and linting setup
- test coverage gaps
- missing integration tests
- missing frontend tests
- missing Rust unit tests
- build scripts
- CI assumptions
- environment/config management
- places where types should be shared or generated

## Required output format

Return exactly these sections:

1. Architecture summary
2. Main organizational problems
3. React findings
4. Rust findings
5. React-Tauri boundary findings
6. Test and tooling gaps
7. Top 10 fixes in priority order
8. Safe refactor plan for the next 3 days
9. Things I should learn first as a Rust/React beginner

## Important

- Be brutally honest.
- Do not hide structural problems.
- Do not rewrite the app yet.
- Start with a read-only review.
