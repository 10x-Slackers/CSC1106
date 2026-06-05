# Agents

## Code Style

- Do not add `derive(Debug)` unless it is actually needed
- Always run `cargo fmt` after completing a task

## DaisyUI Styling Guidelines

- Prefer DaisyUI components over Tailwind utilities. Use `btn`, `card`, `fieldset`, `hero`, `alert`, `input`, `label`, `navbar`, etc. Don't replicate them with Tailwind.
- Use DaisyUI color names (`primary`, `secondary`, `accent`, `base-100`, `error`, etc.) over raw Tailwind.
- When doing HTML/UI design, reference: https://daisyui.com/llms.txt
