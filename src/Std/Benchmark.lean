/-
A small benchmarking harness for Lean programs.

It provides warmup runs, automatic run count determination, summary statistics,
and multiple output formats (pretty, JSON, radar-compatible).
-/
module

prelude
public import Init.Prelude
public import Init.Data.Float
public import Init.Data.OfScientific
public import Init.System.IO
public import Std.Time
public import Std.Internal.Async.Process
public import Init.Data.String.Legacy
public import Init.Data.String.TakeDrop
public import Init.Data.String.Search
public import Init.Data.Array.QSort.Basic

@[expose] public section

namespace Std
namespace Benchmark

open Std.Internal.IO.Process
open Std.Time

/-! ## Types -/

inductive OutputFormat where
  | pretty
  | json
  | radar
  deriving DecidableEq

structure Config where
  warmupRuns : Nat := 1
  minRuns : Nat := 10
  maxRuns : Option Nat := none
  runs : Option Nat := none
  minTime : Float := 1.0
  maxTime : Option Float := none
  format : OutputFormat := .pretty
  outFile? : Option System.FilePath := none
  showProgress : Bool := true
  radarTopic? : Option String := none
  allocprof : Bool := false

structure Bench where
  name : String
  action : IO Unit
  setup : IO Unit := pure ()
  teardown : IO Unit := pure ()
  beforeAll : IO Unit := pure ()
  afterAll : IO Unit := pure ()

structure Sample where
  wallSeconds : Float
  cpuUserSeconds : Float
  cpuSystemSeconds : Float
  heartbeats : Nat
  maxRssDeltaKb : Int
  deriving Inhabited

structure Stats where
  mean : Float
  stddev? : Option Float
  median : Float
  min : Float
  max : Float
  deriving Inhabited

structure BenchResult where
  name : String
  samples : Array Sample
  wall : Stats
  cpuUser : Stats
  cpuSystem : Stats
  heartbeats : Stats
  maxRssDeltaKb : Stats
  deriving Inhabited

/-! ## Utilities -/

def natToFloat (n : Nat) : Float :=
  Float.ofNat n

def intToFloat (n : Int) : Float :=
  Float.ofInt n

def formatFloat (f : Float) : String :=
  toString f

def formatDuration (seconds : Float) : String :=
  if seconds < 1e-6 then
    s!"{formatFloat (seconds * 1e9)} ns"
  else if seconds < 1e-3 then
    s!"{formatFloat (seconds * 1e6)} us"
  else if seconds < 1.0 then
    s!"{formatFloat (seconds * 1e3)} ms"
  else
    s!"{formatFloat seconds} s"

def padRight (s : String) (n : Nat) : String :=
  if s.length >= n then s else s ++ String.ofList (List.replicate (n - s.length) ' ')

def arrayMean (xs : Array Float) : Float :=
  if xs.isEmpty then 0.0 else
    xs.foldl (fun acc x => acc + x) 0.0 / natToFloat xs.size

def arrayStddev? (xs : Array Float) (mean : Float) : Option Float :=
  if xs.size <= 1 then none
  else
    let n := natToFloat (xs.size - 1)
    let var :=
      xs.foldl (fun acc x =>
        let d := x - mean
        acc + d * d
      ) 0.0 / n
    some (Float.sqrt var)

def arrayMedian (xs : Array Float) : Float :=
  if xs.isEmpty then 0.0 else
    let sorted := xs.qsort (fun a b => a < b)
    let n := sorted.size
    if n % 2 == 1 then
      sorted[(n / 2)]!
    else
      let a := sorted[(n / 2) - 1]!
      let b := sorted[(n / 2)]!
      (a + b) / 2.0

def arrayMin (xs : Array Float) : Float :=
  xs.foldl (fun acc x => if x < acc then x else acc) (xs.getD 0 0.0)

def arrayMax (xs : Array Float) : Float :=
  xs.foldl (fun acc x => if x > acc then x else acc) (xs.getD 0 0.0)

def statsOf (xs : Array Float) : Stats :=
  let mean := arrayMean xs
  {
    mean
    stddev? := arrayStddev? xs mean
    median := arrayMedian xs
    min := arrayMin xs
    max := arrayMax xs
  }

def sampleWallSeconds (t0 t1 : Nat) : Float :=
  natToFloat (t1 - t0) / 1e9

def sampleCpuSeconds (before after : Time.Millisecond.Offset) : Float :=
  let diff := after.val - before.val
  intToFloat diff / 1000.0

def sampleMaxRssDeltaKb (before after : UInt64) : Int :=
  Int.ofNat (UInt64.toNat after) - Int.ofNat (UInt64.toNat before)

/-! ## Measurement -/

def runOnce (cfg : Config) (bench : Bench) (runLabel : String) (useAllocprof := cfg.allocprof) :
    IO Sample := do
  bench.setup
  let hb0 ← IO.getNumHeartbeats
  let ru0 ← getResourceUsage
  let t0 ← IO.monoNanosNow
  if useAllocprof then
    allocprof runLabel bench.action
  else
    bench.action
  let t1 ← IO.monoNanosNow
  let ru1 ← getResourceUsage
  let hb1 ← IO.getNumHeartbeats
  bench.teardown
  return {
    wallSeconds := sampleWallSeconds t0 t1
    cpuUserSeconds := sampleCpuSeconds ru0.cpuUserTime ru1.cpuUserTime
    cpuSystemSeconds := sampleCpuSeconds ru0.cpuSystemTime ru1.cpuSystemTime
    heartbeats := hb1 - hb0
    maxRssDeltaKb := sampleMaxRssDeltaKb ru0.peakResidentSetSizeKb ru1.peakResidentSetSizeKb
  }

def shouldContinue (cfg : Config) (runs : Nat) (elapsed : Float) : Bool :=
  let maxTime := cfg.maxTime.getD 1.0e30
  if let some maxRuns := cfg.maxRuns then
    runs < maxRuns && elapsed < maxTime
  else
    elapsed < maxTime

def withProgress (cfg : Config) (msg : String) : IO Unit := do
  if cfg.showProgress then
    IO.eprint s!"\r{msg}"

def clearProgress (cfg : Config) : IO Unit := do
  if cfg.showProgress then
    IO.eprint "\r"

def runBench (cfg : Config) (bench : Bench) : IO BenchResult := do
  bench.beforeAll
  for i in [:cfg.warmupRuns] do
    let label := s!"{bench.name} warmup {i + 1}/{cfg.warmupRuns}"
    withProgress cfg label
    discard <| runOnce cfg bench label false
  let mut samples : Array Sample := #[]
  let benchStart ← IO.monoNanosNow
  let mut elapsed : Float := 0.0
  let mut runs := 0
  let targetRuns :=
    match cfg.runs with
    | some n => n
    | none => cfg.minRuns
  while runs < targetRuns do
    let label := s!"{bench.name} run {runs + 1}"
    withProgress cfg label
    let sample ← runOnce cfg bench label
    samples := samples.push sample
    runs := runs + 1
    let now ← IO.monoNanosNow
    elapsed := sampleWallSeconds benchStart now
  if cfg.runs.isNone then
    while elapsed < cfg.minTime && shouldContinue cfg runs elapsed do
      let label := s!"{bench.name} run {runs + 1}"
      withProgress cfg label
      let sample ← runOnce cfg bench label
      samples := samples.push sample
      runs := runs + 1
      let now ← IO.monoNanosNow
      elapsed := sampleWallSeconds benchStart now
  clearProgress cfg
  bench.afterAll
  let wallStats := statsOf (samples.map (·.wallSeconds))
  let cpuUserStats := statsOf (samples.map (·.cpuUserSeconds))
  let cpuSystemStats := statsOf (samples.map (·.cpuSystemSeconds))
  let heartbeatsStats := statsOf (samples.map (fun s => natToFloat s.heartbeats))
  let maxRssStats := statsOf (samples.map (fun s => intToFloat s.maxRssDeltaKb))
  return {
    name := bench.name
    samples
    wall := wallStats
    cpuUser := cpuUserStats
    cpuSystem := cpuSystemStats
    heartbeats := heartbeatsStats
    maxRssDeltaKb := maxRssStats
  }

/-! ## Reporting -/

def jsonEscape (s : String) : String :=
  s.toList.foldl (fun acc c =>
    match c with
    | '"' => acc ++ "\\\""
    | '\\' => acc ++ "\\\\"
    | '\n' => acc ++ "\\n"
    | '\r' => acc ++ "\\r"
    | '\t' => acc ++ "\\t"
    | _ => acc.push c
  ) ""

def jsonStr (s : String) : String :=
  "\"" ++ jsonEscape s ++ "\""

def jsonNum (f : Float) : String :=
  if f.isNaN || f.isInf then
    "0"
  else
    toString f

def jsonNat (n : Nat) : String :=
  toString n

def jsonObj (fields : List (String × String)) : String :=
  let parts := fields.map (fun (k, v) => jsonStr k ++ ":" ++ v)
  "{" ++ String.intercalate "," parts ++ "}"

def jsonArr (items : Array String) : String :=
  "[" ++ String.intercalate "," items.toList ++ "]"

def statsToJson (s : Stats) : String :=
  let stddev := s.stddev?.getD 0.0
  jsonObj [
    ("mean", jsonNum s.mean),
    ("stddev", jsonNum stddev),
    ("median", jsonNum s.median),
    ("min", jsonNum s.min),
    ("max", jsonNum s.max)
  ]

def benchResultToJson (r : BenchResult) : String :=
  jsonObj [
    ("name", jsonStr r.name),
    ("runs", jsonNat r.samples.size),
    ("wall", statsToJson r.wall),
    ("cpuUser", statsToJson r.cpuUser),
    ("cpuSystem", statsToJson r.cpuSystem),
    ("heartbeats", statsToJson r.heartbeats),
    ("maxRssDeltaKb", statsToJson r.maxRssDeltaKb)
  ]

def resultsToJson (rs : Array BenchResult) : String :=
  jsonObj [("benchmarks", jsonArr (rs.map benchResultToJson))]

def printPretty (rs : Array BenchResult) : IO Unit := do
  if rs.isEmpty then return ()
  let fastest := rs.foldl (fun acc r => min acc r.wall.mean) rs[0]!.wall.mean
  let nameWidth := rs.foldl (fun acc r => Nat.max acc r.name.length) "Benchmark".length
  IO.println <| String.intercalate "  " #[
    padRight "Benchmark" nameWidth,
    padRight "Time (mean ± σ)" 24,
    padRight "Min ... Max" 20,
    padRight "Runs" 6,
    "Relative"
  ].toList
  for r in rs do
    let meanStr := formatDuration r.wall.mean
    let stddevStr :=
      match r.wall.stddev? with
      | some s => s!" ± {formatDuration s}"
      | none => ""
    let minMaxStr := s!"{formatDuration r.wall.min} ... {formatDuration r.wall.max}"
    let rel :=
      if fastest == 0.0 then "n/a"
      else
        let ratio := r.wall.mean / fastest
        if ratio == 1.0 then "1.00x"
        else s!"{formatFloat ratio}x"
    IO.println <| String.intercalate "  " #[
      padRight r.name nameWidth,
      padRight (meanStr ++ stddevStr) 24,
      padRight minMaxStr 20,
      padRight (toString r.samples.size) 6,
      rel
    ].toList

def printRadar (cfg : Config) (rs : Array BenchResult) : IO Unit := do
  let topicPrefix := cfg.radarTopic?.getD "bench"
  for r in rs do
    let metric := s!"{topicPrefix}/{r.name}//wall-clock"
    let obj := jsonObj [
      ("metric", jsonStr metric),
      ("value", jsonNum r.wall.mean),
      ("unit", jsonStr "s")
    ]
    IO.println s!"radar::measurement={obj}"

def writeJson (path : System.FilePath) (rs : Array BenchResult) : IO Unit := do
  IO.FS.writeFile path (resultsToJson rs)

def outputResults (cfg : Config) (rs : Array BenchResult) : IO Unit := do
  match cfg.format with
  | .pretty => printPretty rs
  | .json =>
      if let some out := cfg.outFile? then
        writeJson out rs
      else
        IO.println (resultsToJson rs)
  | .radar => printRadar cfg rs

/-! ## CLI parsing -/

def parseNat (s : String) : Except String Nat :=
  match s.toNat? with
  | some n => .ok n
  | none => .error s!"expected Nat, got '{s}'"

def parseFloat (s : String) : Except String Float := do
  let parts := s.splitOn "."
  match parts with
  | [whole] =>
      let n ← parseNat whole
      return natToFloat n
  | [whole, frac] =>
      let n ← parseNat whole
      let f ← parseNat frac
      let scale := Nat.pow 10 frac.length
      return natToFloat n + (natToFloat f / natToFloat scale)
  | _ => .error s!"invalid float '{s}'"

def parseDuration (s : String) : Except String Float := do
  let (numStr, unit) :=
    if s.endsWith "ms" then ((s.dropEnd 2).toString, "ms")
    else if s.endsWith "us" then ((s.dropEnd 2).toString, "us")
    else if s.endsWith "ns" then ((s.dropEnd 2).toString, "ns")
    else if s.endsWith "s" then ((s.dropEnd 1).toString, "s")
    else (s, "s")
  let val ← parseFloat numStr
  match unit with
  | "ms" => return val / 1000.0
  | "us" => return val / 1000000.0
  | "ns" => return val / 1000000000.0
  | _ => return val

public def Config.parseArgs (args : List String) : Except String (Config × List String) := do
  let args := match args with
    | "--" :: xs => xs
    | xs => xs
  let rec go (cfg : Config) (rest : List String) : Except String (Config × List String) := do
    match rest with
    | [] => return (cfg, [])
    | "--" :: xs => return (cfg, xs)
    | arg :: xs =>
      if arg == "--json" then
        go {cfg with format := .json, showProgress := false} xs
      else if arg == "--radar" then
        go {cfg with format := .radar, showProgress := false} xs
      else if arg == "--no-progress" then
        go {cfg with showProgress := false} xs
      else if arg == "--allocprof" then
        go {cfg with allocprof := true} xs
      else if arg.startsWith "--runs=" then
        let n ← parseNat (arg.drop 7 |>.toString)
        go {cfg with runs := some n} xs
      else if arg == "--runs" then
        match xs with
        | v :: xs' =>
          let n ← parseNat v
          go {cfg with runs := some n} xs'
        | [] => .error "missing value for --runs"
      else if arg.startsWith "--min-runs=" then
        let n ← parseNat (arg.drop 11 |>.toString)
        go {cfg with minRuns := n} xs
      else if arg == "--min-runs" then
        match xs with
        | v :: xs' =>
          let n ← parseNat v
          go {cfg with minRuns := n} xs'
        | [] => .error "missing value for --min-runs"
      else if arg.startsWith "--max-runs=" then
        let n ← parseNat (arg.drop 11 |>.toString)
        go {cfg with maxRuns := some n} xs
      else if arg == "--max-runs" then
        match xs with
        | v :: xs' =>
          let n ← parseNat v
          go {cfg with maxRuns := some n} xs'
        | [] => .error "missing value for --max-runs"
      else if arg.startsWith "--warmup=" then
        let n ← parseNat (arg.drop 9 |>.toString)
        go {cfg with warmupRuns := n} xs
      else if arg == "--warmup" then
        match xs with
        | v :: xs' =>
          let n ← parseNat v
          go {cfg with warmupRuns := n} xs'
        | [] => .error "missing value for --warmup"
      else if arg.startsWith "--min-time=" then
        let v ← parseDuration (arg.drop 11 |>.toString)
        go {cfg with minTime := v} xs
      else if arg == "--min-time" then
        match xs with
        | v :: xs' =>
          let t ← parseDuration v
          go {cfg with minTime := t} xs'
        | [] => .error "missing value for --min-time"
      else if arg.startsWith "--max-time=" then
        let v ← parseDuration (arg.drop 11 |>.toString)
        go {cfg with maxTime := some v} xs
      else if arg == "--max-time" then
        match xs with
        | v :: xs' =>
          let t ← parseDuration v
          go {cfg with maxTime := some t} xs'
        | [] => .error "missing value for --max-time"
      else if arg.startsWith "--format=" then
        match (arg.drop 9).toString with
        | "pretty" => go {cfg with format := .pretty} xs
        | "json" => go {cfg with format := .json, showProgress := false} xs
        | "radar" => go {cfg with format := .radar, showProgress := false} xs
        | _ => .error s!"unknown format '{(arg.drop 9).toString}'"
      else if arg == "--format" then
        match xs with
        | v :: xs' =>
          match v with
          | "pretty" => go {cfg with format := .pretty} xs'
          | "json" => go {cfg with format := .json, showProgress := false} xs'
          | "radar" => go {cfg with format := .radar, showProgress := false} xs'
          | _ => .error s!"unknown format '{v}'"
        | [] => .error "missing value for --format"
      else if arg.startsWith "--output=" then
        let path := System.FilePath.mk (arg.drop 9 |>.toString)
        go {cfg with outFile? := some path, format := .json, showProgress := false} xs
      else if arg == "--output" then
        match xs with
        | v :: xs' =>
          go {cfg with outFile? := some (System.FilePath.mk v), format := .json, showProgress := false} xs'
        | [] => .error "missing value for --output"
      else if arg.startsWith "--radar-topic=" then
        let topic := (arg.drop 14).toString
        go {cfg with radarTopic? := some topic} xs
      else if arg == "--radar-topic" then
        match xs with
        | v :: xs' => go {cfg with radarTopic? := some v} xs'
        | [] => .error "missing value for --radar-topic"
      else
        return (cfg, arg :: xs)
  go {} args

/-! ## Runner -/

public def run (cfg : Config) (benches : Array Bench) : IO (Array BenchResult) := do
  let mut results : Array BenchResult := #[]
  for bench in benches do
    results := results.push (← runBench cfg bench)
  return results

public def runAndReport (cfg : Config) (benches : Array Bench) : IO Unit := do
  let results ← run cfg benches
  outputResults cfg results

public def main (benches : Array Bench) (args : List String) : IO UInt32 := do
  match Config.parseArgs args with
  | .error err =>
      IO.eprintln s!"bench: {err}"
      return 1
  | .ok (cfg, _rest) =>
      runAndReport cfg benches
      return 0

end Benchmark
end Std
