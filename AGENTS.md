# Agents

## Research First

- Always research before implementing a task — both the codebase and online. Understand existing patterns, dependencies, and conventions before writing code.
- Refer to the latest documentation and existing implementations of any libraries or frameworks in use.
- Check `docs/` for architectural and planning details relevant to the task.

## Ask When Unsure

- Clarify ambiguities before proceeding — ask rather than guess.
- When in doubt about scope, priorities, or critical decisions, surface the uncertainty and let the user decide.

## Code Style

- Do not add `derive(Debug)` unless it is actually needed
  - Run `cargo clippy` to check for unused `Debug` derives and remove them
  - SeaORM `DeriveEntityModel` requires `Debug`
- Always run `cargo fmt` after completing a task

## DaisyUI Styling Guidelines

- Prefer DaisyUI components over Tailwind utilities. Use `btn`, `card`, `fieldset`, `hero`, `alert`, `input`, `label`, `navbar`, etc. Don't replicate them with Tailwind.
- Use DaisyUI color names (`primary`, `secondary`, `accent`, `base-100`, `error`, etc.) over raw Tailwind.
- When doing HTML/UI design, reference: https://daisyui.com/llms.txt
