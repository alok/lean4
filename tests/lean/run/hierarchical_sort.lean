set_option pp.universes true

/-- info: Prop @1 : Type @1 -/
#guard_msgs in
#check (Sort 0 @ 1)

/-- info: Type @2 : Type 1 @2 -/
#guard_msgs in
#check (Type 0 @ 2)
