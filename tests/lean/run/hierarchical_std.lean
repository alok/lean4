set_option pp.universes true

universe u v h

/-! Prototype nonstandard API (temporary; will move to Init.Nonstandard after stage0 update). -/

axiom HLift.{r, u1, h1} (α : Type u1 @ h1) : Type u1 @ (max h1 r)
axiom HLift.up.{r, u1, h1} {α : Type u1 @ h1} : α → HLift.{r, u1, h1} α
axiom HLift.down.{r, u1, h1} {α : Type u1 @ h1} : HLift.{r, u1, h1} α → α

axiom Std {α : Sort u @ h} : α → Prop @ (h+1)
axiom Internal {α : Sort u @ h} (p : α) : Prop @ (h+1)
axiom StdFinite {α : Sort u @ h} (s : α) : Prop @ (h+1)
axiom Transfer {α : Sort u @ h} {p : α → Prop @ h} :
    Internal p → (∀ x : α, Std x → p x) → ∀ x : α, p x

/-- info: HLift.{1, 0, 0} Nat : Type @1 -/
#guard_msgs in
#check (HLift.{1} Nat)

/-- info: Std.{1, 0} 0 : Prop @1 -/
#guard_msgs in
#check (Std (0 : Nat))

/-- info: Std.{2, 0} Nat : Prop @1 -/
#guard_msgs in
#check (Std Nat)

/-- info: Std.{2, 1} (HLift.{1, 0, 0} Nat) : Prop @2 -/
#guard_msgs in
#check (Std (HLift.{1} Nat))
