# Restrictions

This file lists every user-facing refusal text the plugin emits and who approved it.

| ID | message | why | approved |
|---|---|---|---|
| TC-BLOCKED-LOG | `blocked screen=<css\|sss> dir=<dir\|mixed> presses=<n> …` (events.log) and the state line `blocked=<n>` | Logs and counts an L3 + direction press that did not become a step attempt: the hand may not step (hand disabled, P1 inactive, or no token carried: the plugin's rule since its first release, by which such a press does nothing), or that update already took its one step. It reports that rule; it adds no restriction and changes no input. | tr-primary-2 compliance disposition, 2026-11-22 (primary ledger 08:03:29Z and 08:39:49Z), under the operator's 03:09Z fix authority |
