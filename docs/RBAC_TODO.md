# RBAC Action-Level TODO

Deferred work for the two-tier RBAC plan. Page-level gates are done (`/users` → `Require<AdminOnly>`, `/reports` → `Require<Finance>`). The items below cover action-level gates, ownership scoping, and template-side enforcement for shared pages.

## Prerequisite

- [x] Verify invoice entity has `created_by_user_id` column — confirmed at `src/entity/invoice.rs:41` (ERD `docs/ERD.mmd:29`). Claims already has `submitted_by_user_id` (`src/entity/claim.rs`). No migration needed.

## Template helper

- [ ] Add `Role::can(roles: &[Role]) -> bool` helper on `crate::entity::user::Role` for template button visibility (void, approve, record payment, party CRUD). Register as a Tera filter or inject via context so templates can do `{% if user.role | can([Admin, Accountant]) %}`.

## Action-level gates (inline check, render forbidden inline — no flash/session)

- [ ] Void invoice — inline `if !user.role.can(&[Role::Admin, Role::Accountant]) { return render_forbidden_action(...) }`
- [ ] Record payment against invoice — same check
- [ ] Approve/reject claim — same check

## Ownership scoping (data-level, enforced in handler/query)

- [ ] Staff invoices list filtered to `created_by_user_id = user.id`; Admin/Accountant see all
- [ ] Staff claims list filtered to `submitted_by_user_id = user.id`; Admin/Accountant see all
- [ ] Staff edit/void invoice — load record, deny if `created_by != user.id`
- [ ] Staff edit/withdraw claim — load record, deny if `submitted_by != user.id`

## Template-side enforcement

- [ ] Party read-only for Staff: hide/disable create/edit/deactivate buttons via `Role::can`

## Future routes

- [ ] Add `/journal` route with `Require<Finance>` gate when manual journal entry handler is built
