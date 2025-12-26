/-
Copyright (c) 2025 Lean FRO LLC. All rights reserved.
Released under Apache 2.0 license as described in the file LICENSE.
Authors: Alok Singh
-/
module

prelude
public import Lean.Attributes

public section

namespace Lean

/--
  Information about a C-compatible struct for FFI.

  - `cName`: The C struct type name (e.g., "vec3f")
  - `size`: Total size in bytes
  - `alignment`: Required memory alignment
  - `fieldNames`: Lean field names in declaration order
  - `fieldCTypes`: C type strings for each field (e.g., "float", "double")
-/
structure CStructInfo where
  cName      : String
  size       : Nat
  alignment  : Nat
  fieldNames : Array Name := #[]
  fieldCTypes : Array String := #[]
  deriving Inhabited, BEq, Hashable

/--
Parser for `@[cstruct "c_name" size alignment]` attribute.

Usage:
```
@[cstruct "vec3f" 12 4]
structure Vec3f where
  x : Float32
  y : Float32
  z : Float32
```
-/
private def syntaxToCStructInfo (stx : Syntax) : AttrM CStructInfo := do
  -- stx[0] is the attribute name 'cstruct'
  -- stx[1] is the args node containing: cName, size, alignment
  let args := stx[1].getArgs
  if args.size < 3 then
    throwErrorAt stx "Expected: @[cstruct \"c_name\" size alignment]"
  let cName ← match args[0]!.isStrLit? with
    | some s => pure s
    | none => throwErrorAt args[0]! "Expected string literal for C struct name"
  let size ← match args[1]!.isNatLit? with
    | some n => pure n
    | none => throwErrorAt args[1]! "Expected natural number for size"
  let alignment ← match args[2]!.isNatLit? with
    | some n => pure n
    | none => throwErrorAt args[2]! "Expected natural number for alignment"
  return { cName, size, alignment }

/-- Extract field information from structure declaration. -/
private def extractStructFields (declName : Name) : AttrM (Array Name × Array String) := do
  let env ← getEnv
  let some (.inductInfo inductInfo) := env.find? declName
    | throwError "Declaration {declName} is not an inductive type"
  unless inductInfo.numCtors == 1 do
    throwError "@[cstruct] can only be applied to single-constructor structures"
  let [ctorName] := inductInfo.ctors
    | throwError "Expected exactly one constructor"
  let some (.ctorInfo ctorInfo) := env.find? ctorName
    | throwError "Constructor {ctorName} not found"

  -- Extract field names from the constructor
  let mut fieldNames : Array Name := #[]
  let mut fieldCTypes : Array String := #[]

  -- For now, we'll populate these when we have more type info
  -- The ToIRType phase will fill in the actual C types
  return (fieldNames, fieldCTypes)

builtin_initialize cstructAttr : ParametricAttribute CStructInfo ←
  registerParametricAttribute {
    name := `cstruct
    descr := "C-compatible struct for FFI. Usage: @[cstruct \"c_name\" size alignment]"
    getParam := fun _ stx => syntaxToCStructInfo stx
    afterSet := fun declName cInfo => do
      let env ← getEnv
      -- Validate that this is applied to a structure
      let some (.inductInfo inductInfo) := env.find? declName
        | throwError "@[cstruct] can only be applied to structures, not {declName}"
      unless inductInfo.isRec == false && inductInfo.numCtors == 1 do
        throwError "@[cstruct] can only be applied to non-recursive single-constructor structures"

      -- Log for debugging
      -- logInfo m!"Registered C struct: {cInfo.cName} (size={cInfo.size}, align={cInfo.alignment})"
  }

/-- Get CStructInfo for a declaration if it has the @[cstruct] attribute. -/
def getCStructInfo? (env : Environment) (declName : Name) : Option CStructInfo :=
  cstructAttr.getParam? env declName

/-- Check if a declaration has the @[cstruct] attribute. -/
def hasCStructAttr (env : Environment) (declName : Name) : Bool :=
  (getCStructInfo? env declName).isSome

/-- Get all declarations with @[cstruct] attribute in the current module. -/
def getAllCStructs (env : Environment) : List (Name × CStructInfo) :=
  let (names, map) := cstructAttr.ext.getState env
  names.filterMap fun n => (map.find? n).map (n, ·)

end Lean
