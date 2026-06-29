# Agents

## Project context

This is a **university coursework project** done by undergraduates. It is not a production system and will never be deployed to real users.

## Research First

- Always research before implementing a task — both the codebase and online. Understand existing patterns, dependencies, and conventions before writing code.
- Refer to the latest documentation and existing implementations of any libraries or frameworks in use.
- Check `docs/` for architectural and planning details relevant to the task.
- When implementing business-rule validation (accounting, financial, domain-specific), verify the rule's correctness against real-world scenarios before implementing.

## Ask When Unsure

- Clarify ambiguities before proceeding — ask rather than guess.
- When in doubt about scope, priorities, or critical decisions, surface the uncertainty and let the user decide.

## Code Style

- Do not add `derive(Debug)` unless it is actually needed
  - SeaORM `DeriveEntityModel` requires `Debug`
- Always run `just format` and `just lint` after completing a task
- Reduce duplicated code/logic. Extract shared helpers when patterns repeat, and account for likely future development where it makes sense
- Don't spam comments or rustdoc. No verbose or decorative comments. Keep rustdoc short and normal comments concise — refer to the existing codebase for style

## Testing

- The server is long-running. Ask the user to start it if not already up; don't start it yourself as it will hang the session

## UI posture

- The app is pure SSR (Tera templates, no client framework). Avoid JavaScript. No client-side calculations or SPA behavior — the server validates and computes everything on submit. Prefer plain HTML forms and full-page reloads.
- Prioritise simplicity over UX. Sacrifice polish (dynamic behavior, persistent UI state) for less code.
- Prefer DaisyUI components over Tailwind utilities. Don't replicate them with Tailwind.
- Use DaisyUI color names over raw Tailwind.
- When doing HTML/UI design, reference: https://daisyui.com/llms.txt
