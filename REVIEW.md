# Review rules

The reviewer reads this file from the base branch, never from the pull request.
Each rule has a trigger that can be seen in a diff, a check, and where the rule came from.
Add a rule when the same problem shows up in two PRs. Delete rules that never fire.

## Always

| Trigger | Check | Origin |
| --- | --- | --- |
| Any PR | Diff stays inside the linked ticket's Scope | Kit default |
| Any PR | PR body shows verify and the ticket's Proof command passing | Kit default |
| New or changed behaviour | A test fails without the change and passes with it | Kit default |
| Test file changed | No test was skipped, deleted, loosened, or given a sleep or retry to pass | Kit default |
| `REVIEW.md`, `.devteam/`, `.github/workflows/` changed | PR carries the `judge-change` label and a human asked for it | Kit default |

## Project rules

| Trigger | Check | Origin |
| --- | --- | --- |
| TODO: e.g. a serialized type changes | Old data still loads | PR #, incident |
