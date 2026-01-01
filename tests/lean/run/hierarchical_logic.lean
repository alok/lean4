set_option pp.universes true

universe u h

variable {α : Sort u @ h} (a b : α)
variable (p q : Prop @ h)

/-- info: Eq a b : Prop @h -/
#guard_msgs in
#check (Eq a b)

/-- info: And p q : Prop @h -/
#guard_msgs in
#check (And p q)

/-- info: Or p q : Prop @h -/
#guard_msgs in
#check (Or p q)

/-- info: Iff p q : Prop @h -/
#guard_msgs in
#check (Iff p q)

/-- info: Not p : Prop @h -/
#guard_msgs in
#check (Not p)

/-- info: Exists (fun _ : α => p) : Prop @h -/
#guard_msgs in
#check (Exists fun _ : α => p)

/-- info: Subtype (fun _ : α => p) : Sort u @h -/
#guard_msgs in
#check (Subtype fun _ : α => p)

/-- info: Decidable p : Type @h -/
#guard_msgs in
#check (Decidable p)
