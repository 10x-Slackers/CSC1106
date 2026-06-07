# Agents

## Research First

- Always research before implementing a task — both the codebase and online. Understand existing patterns, dependencies, and conventions before writing code.
- Refer to the latest documentation and existing implementations of any libraries or frameworks in use.
- Check `docs/` for architectural and planning details relevant to the task.

## Code Style

- Do not add `derive(Debug)` unless it is actually needed
- Always run `cargo fmt` after completing a task

## DaisyUI Styling Guidelines

- Prefer DaisyUI components over Tailwind utilities. Use `btn`, `card`, `fieldset`, `hero`, `alert`, `input`, `label`, `navbar`, etc. Don't replicate them with Tailwind.
- Use DaisyUI color names (`primary`, `secondary`, `accent`, `base-100`, `error`, etc.) over raw Tailwind.
- When doing HTML/UI design, reference: https://daisyui.com/llms.txt
