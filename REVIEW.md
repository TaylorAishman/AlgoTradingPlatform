# Review rules

The reviewer reads this file from the default branch, never from the chain under review.
It reviews a chain one commit at a time. Each rule has a trigger that can be seen in a
diff, a check, and where the rule came from.
Add a rule when the same problem shows up in two chains. Delete rules that never fire.

## Always

| Trigger | Check | Origin |
| --- | --- | --- |
| Any commit | Subject is `[<ID>] <step>` and the diff stays inside that step's Files | Kit default |
| Any commit | It does one thing and passes on its own: reverting it removes exactly that step | Kit default |
| New or changed behaviour | A test asserts the requirement's acceptance check | Kit default |
| Test file changed | No test was skipped, deleted, loosened, or given a sleep or retry to pass | Kit default |
| `REVIEW.md`, `.devteam/`, `.github/workflows/` changed in a chain | Flag it: only the integrator or a human changes these | Kit default |
| The chain as a whole | The commits tell the chain's Story in an order a reader can follow | Kit default |

## Project rules

| Trigger | Check | Origin |
| --- | --- | --- |
| TODO: e.g. a serialized type changes | Old data still loads | chain name, incident |
