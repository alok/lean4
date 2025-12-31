import Std

open Std

private def consume (sink : IO.Ref Nat) (x : Nat) : IO Unit := do
  sink.set x

private def benchArrayFold (sink : IO.Ref Nat) : IO Unit := do
  let arr := Array.range 100000
  let sum := arr.foldl (fun acc x => acc + x) 0
  consume sink sum

private def benchHashMap (sink : IO.Ref Nat) : IO Unit := do
  let mut m : Std.HashMap Nat Nat := {}
  for i in [:50000] do
    m := m.insert i (i + 1)
  let mut acc := 0
  for i in [:50000] do
    if let some v := m.get? i then
      acc := acc + v
  consume sink acc

private def benchArrayQSort (sink : IO.Ref Nat) : IO Unit := do
  let arr := Array.range 20000
  let sorted := arr.reverse.qsort (fun a b => a < b)
  let last := sorted[sorted.size - 1]!
  consume sink (sorted[0]! + last)

private def benchArrayMap (sink : IO.Ref Nat) : IO Unit := do
  let arr := Array.range 100000
  let mapped := arr.map (fun x => x * 2)
  consume sink mapped.size

private def benchStringSplit (sink : IO.Ref Nat) : IO Unit := do
  let parts := List.replicate 1000 "alpha,beta,gamma,delta,epsilon"
  let big := String.intercalate "," parts
  let pieces := big.splitOn ","
  consume sink pieces.length

private def mkBenches (sink : IO.Ref Nat) : Array Std.Benchmark.Bench :=
  #[
    { name := "arrayFold", action := benchArrayFold sink },
    { name := "hashMap", action := benchHashMap sink },
    { name := "arrayQSort", action := benchArrayQSort sink },
    { name := "arrayMap", action := benchArrayMap sink },
    { name := "stringSplit", action := benchStringSplit sink }
  ]

def main (args : List String) : IO UInt32 := do
  let sink ← IO.mkRef 0
  Std.Benchmark.main (mkBenches sink) args
