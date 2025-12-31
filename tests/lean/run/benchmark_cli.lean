import Std

open Std.Benchmark

private def assertTrue (cond : Bool) (msg : String) : IO Unit :=
  if cond then
    pure ()
  else
    throw (IO.userError msg)

private def expectFormat (cfg : Config) (expected : OutputFormat) : IO Unit :=
  match cfg.format, expected with
  | .pretty, .pretty => pure ()
  | .json, .json => pure ()
  | .radar, .radar => pure ()
  | .prettyFull, .prettyFull => pure ()
  | _, _ =>
      throw (IO.userError "unexpected format")

private def parseOrDie (args : List String) : IO Config :=
  match Config.parseArgs args with
  | .ok (cfg, rest) => do
      assertTrue rest.isEmpty s!"expected no remaining args, got {rest}"
      pure cfg
  | .error err =>
      throw (IO.userError err)

#eval do
  let cfg ← parseOrDie ["--format=pretty-full"]
  expectFormat cfg .prettyFull

#eval do
  let cfg ← parseOrDie ["--format", "full"]
  expectFormat cfg .prettyFull

#eval do
  let cfg ← parseOrDie ["--format=radar"]
  expectFormat cfg .radar
  assertTrue (!cfg.showProgress) "radar format should disable progress"

#eval do
  let cfg ← parseOrDie ["--output", "bench.json"]
  expectFormat cfg .json
  assertTrue (!cfg.showProgress) "json output should disable progress"
  assertTrue (cfg.outFile? == some (System.FilePath.mk "bench.json")) "output path mismatch"

#eval do
  let cfg ← parseOrDie ["--max-runs", "12", "--min-runs", "5"]
  assertTrue (cfg.maxRuns == some 12) "max-runs mismatch"
  assertTrue (cfg.minRuns == 5) "min-runs mismatch"
