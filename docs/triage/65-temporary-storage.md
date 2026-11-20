# Triage: use temporary storage for recomputable data (#65)

## Checked

- Every read and write in `src/lib.rs` uses `env.storage().instance()`. Nothing uses the temporary tier.
- Payments, VASP records and thresholds are all stored under `instance()`.

## Why no change was made

- Temporary entries expire. Moving an index there means it disappears after its TTL unless it is extended, as the TTL regression on the nasrin105 branch of PR #132 showed.
- A rebuild function needs a list of the IDs to rebuild from. The only list is the index itself, so the index cannot be deleted and rebuilt from payment records alone. The contract has a payment counter, but it does not enumerate payment IDs.
- Payment records themselves must stay durable, so only derived data can move. Which entries are derived needs a decision from the maintainers.

## Next steps

1. Decide which derived entries can expire, and accept that they will need a rebuild path.
2. Keep one durable list of payment IDs, or accept a rebuild that scans a bounded range of IDs.
3. Add a test that the rebuild restores each index.

## Not changed

No code change was made. This note records the constraint, not a fix.
