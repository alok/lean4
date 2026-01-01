# Hierarchical type theory (experiment)

This document describes the initial kernel-level experiment that adds a second
universe axis ("hierarchy level") to Lean's `Sort`.

## Syntax

- `Sort u @ h`
- `Type u @ h` (shorthand for `Sort (u+1) @ h`)
- `Prop @ h` (shorthand for `Sort 0 @ h`)

When `@ h` is omitted, `h` defaults to `0`.

## Core typing rules

Let `u, v, h, k : Level`.

- **Sort:** `Sort u @ h : Sort (u+1) @ h`
- **Pi:** If `A : Sort u @ h` and `B : Sort v @ k` then
  `(forall x : A, B) : Sort (imax u v) @ max h k`
- **Definitional equality of sorts:**
  `Sort u @ h` is definitionally equal to `Sort v @ k` iff
  `u` is defeq to `v` and `h` is defeq to `k`.

`Prop` is recognized by `u = 0` (the hierarchy level does not affect that
predicate).

## Kernel changes

- `Expr.sort` now stores two `Level`s: the universe level and the hierarchy
  level.
- Hashing and `hasLevelMVar/hasLevelParam` for sorts incorporate both levels.
- The kernel type checker:
  - Preserves `h` in the `Sort` typing rule.
  - Uses `max` on hierarchy levels when computing a Pi type.
  - Requires defeq on both levels when comparing sorts.
- Inductive declarations:
  - All mutually inductive types must agree on both levels.
  - Constructor argument types must satisfy:
    - `u` constraint as before (or be Prop), and
    - `h` is bounded by the inductive's result `h`.
  - Recursor motives use the inductive's result hierarchy level.

## Parser/elaborator changes

- `Type`, `Sort`, and `Prop` accept an optional `@ h`.
- The pretty printer emits `@ h` when `h` is nonzero.
- Level substitution, abstraction, normalization, and collection traverse both
  levels in a sort.
- Meta-level type inference and definitional equality propagate and compare
  `h` as described above.

## Library support (experimental)

- Prototype `HLift` and nonstandard axioms (`Std`, `Transfer`, `Idealization`,
  `Standardization`) currently live in
  `tests/lean/run/hierarchical_std.lean`.
- These are intentionally minimal placeholders for HST-style development and
  will move into `Init` once stage0 is updated to parse `@ h` syntax in core
  modules.

## Performance notes

- The common case `h = 0` keeps the old hash shape for `Sort` and avoids
  additional level defeq checks.
- Sort equality and equivalence checks short-circuit when hierarchy levels are
  pointer-equal or both zero.
- Pretty-printing omits `@ 0` to keep output identical in the standard case.

## Open questions / next steps

- Add a standardness predicate and the associated axioms for hierarchical
  nonstandard reasoning.
- Decide whether hierarchy levels should be cumulative or strictly stratified.
- Revisit the impredicativity policy for `Prop @ h` across hierarchy levels.
