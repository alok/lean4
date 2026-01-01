/-
Copyright (c) 2025 Lean FRO, LLC. All rights reserved.
Released under Apache 2.0 license as described in the file LICENSE.
Authors: Cameron Zwarich
-/
module

prelude
public import Lean.Compiler.ClosedTermCache
public import Lean.Compiler.NeverExtractAttr
public import Lean.Compiler.LCNF.Internalize
public import Lean.Compiler.LCNF.ToExpr
import Lean.Util.ForEachExpr
import Lean.Util.ForEachExprWhere

public section

namespace Lean.Compiler.LCNF
namespace ExtractClosed

structure ExtractState where
  visited : FVarIdHashSet := {}
  decls   : Array CodeDecl := #[]

abbrev ExtractM := StateRefT ExtractState CompilerM

mutual

partial def extractExpr (e : Expr) : ExtractM Unit :=
  e.forEachWhere Expr.isFVar fun e => extractFVar e.fvarId!

partial def extractLetValue (v : LetValue) : ExtractM Unit := do
  match v with
  | .const _ _ args => args.forM extractArg
  | .fvar fnVar args =>
    extractFVar fnVar
    args.forM extractArg
  | .proj _ _ baseVar => extractFVar baseVar
  | .reset _ fvarId => extractFVar fvarId
  | .reuse fvarId _ _ _ args => extractFVar fvarId; args.forM extractArg
  | .set fvarId _ val => extractFVar fvarId; extractArg val
  | .uset fvarId _ val => extractFVar fvarId; extractFVar val
  | .sset fvarId _ _ val ty => extractFVar fvarId; extractFVar val; extractExpr ty
  | .lit _ | .erased => return ()

partial def extractArg (arg : Arg) : ExtractM Unit := do
  match arg with
  | .fvar fvarId => extractFVar fvarId
  | .type e => extractExpr e
  | .erased => return ()

partial def extractFVar (fvarId : FVarId) : ExtractM Unit := do
  if (← get).visited.contains fvarId then
    return ()
  modify fun s => { s with visited := s.visited.insert fvarId }
  if let some letDecl ← findLetDecl? fvarId then
    extractLetValue letDecl.value
    modify fun s => { s with decls := s.decls.push (.let letDecl) }

end

def isIrrelevantArg (arg : Arg) : Bool :=
  match arg with
  | .erased | .type _ => true
  | .fvar _ => false

structure Context where
  baseName : Name
  sccDecls : Array Decl

structure State where
  decls : Array Decl := {}

abbrev M := ReaderT Context $ StateRefT State CompilerM

mutual

partial def shouldExtractExpr (e : Expr) : M Bool := do
  if !e.hasFVar then return true
  match e with
  | .fvar fvarId => shouldExtractFVar fvarId
  | .app f a => return (← shouldExtractExpr f) && (← shouldExtractExpr a)
  | .lam _ d b _ | .forallE _ d b _ => return (← shouldExtractExpr d) && (← shouldExtractExpr b)
  | .letE _ t v b _ => return (← shouldExtractExpr t) && (← shouldExtractExpr v) && (← shouldExtractExpr b)
  | .mdata _ b | .proj _ _ b => shouldExtractExpr b
  | _ => return true

partial def shouldExtractLetValue (isRoot : Bool) (v : LetValue) : M Bool := do
  match v with
  | .lit (.str _) => return true
  | .lit (.nat v) =>
    -- The old compiler's implementation used the runtime's `is_scalar` function, which
    -- introduces a dependency on the architecture used by the compiler.
    return !isRoot || v >= Nat.pow 2 63
  | .const name _ args =>
    if (← read).sccDecls.any (·.name == name) then
      return false
    if hasNeverExtractAttribute (← getEnv) name then
      return false
    if isRoot then
      if let some constInfo := (← getEnv).find? name then
        let shouldExtract := match constInfo with
        | .defnInfo val => val.type.isForall
        | .ctorInfo _ => !(args.all isIrrelevantArg)
        | _ => true
        if !shouldExtract then
          return false
    args.allM shouldExtractArg
  | .fvar fnVar args => return (← shouldExtractFVar fnVar) && (← args.allM shouldExtractArg)
  | .proj _ _ baseVar => shouldExtractFVar baseVar
  | .reset _ fvarId => shouldExtractFVar fvarId
  | .reuse fvarId _ _ _ args => return (← shouldExtractFVar fvarId) && (← args.allM shouldExtractArg)
  | .set fvarId _ val => return (← shouldExtractFVar fvarId) && (← shouldExtractArg val)
  | .uset fvarId _ val => return (← shouldExtractFVar fvarId) && (← shouldExtractFVar val)
  | .sset fvarId _ _ val ty => return (← shouldExtractFVar fvarId) && (← shouldExtractFVar val) && (← shouldExtractExpr ty)
  | .lit _ | .erased => return !isRoot

partial def shouldExtractArg (arg : Arg) : M Bool := do
  match arg with
  | .fvar fvarId => shouldExtractFVar fvarId
  | .type e => shouldExtractExpr e
  | .erased => return true

partial def shouldExtractFVar (fvarId : FVarId) : M Bool := do
  if let some letDecl ← findLetDecl? fvarId then
    shouldExtractLetValue false letDecl.value
  else
    return false

end

mutual

partial def visitCode (code : Code) : M Code := do
  match code with
  | .let decl k =>
    if (← shouldExtractLetValue true decl.value) then
      let ⟨_, s⟩ ← extractLetValue decl.value |>.run {}
      let decls := s.decls.push (.let decl)
      let decls ← decls.mapM Internalize.internalizeCodeDecl |>.run' {}
      let closedCode := attachCodeDecls decls (.return decls.back!.fvarId)
      if closedCode.toExpr.hasFVar then
        -- This should not happen if shouldExtractLetValue is correct
        return code.updateLet! decl (← visitCode k)
      let closedExpr := closedCode.toExpr
      let env ← getEnv
      let name ← if let some closedTermName := getClosedTermName? env closedExpr then
        eraseCode closedCode
        pure closedTermName
      else
        let name := (← read).baseName ++ (`_closed).appendIndexAfter (← get).decls.size
        cacheClosedTermName env closedExpr name |> setEnv
        let decl := { name, levelParams := [], type := decl.type, params := #[],
                      value := .code closedCode, inlineAttr? := some .noinline }
        decl.saveMono
        modify fun s => { s with decls := s.decls.push decl }
        pure name
      let decl ← decl.updateValue (.const name [] #[])
      return code.updateLet! decl (← visitCode k)
    else
      return code.updateLet! decl (← visitCode k)
  | .fun decl k =>
    let decl ← decl.updateValue (← visitCode decl.value)
    return code.updateFun! decl (← visitCode k)
  | .jp decl k =>
    let decl ← decl.updateValue (← visitCode decl.value)
    return code.updateFun! decl (← visitCode k)
  | .cases cases =>
    let alts ← cases.alts.mapM (fun alt => do return alt.updateCode (← visitCode alt.getCode))
    return code.updateAlts! alts
  | .jmp .. | .return _ | .unreach .. => return code

end

def visitDecl (decl : Decl) : M Decl := do
  let value ← decl.value.mapCodeM visitCode
  return { decl with value }

end ExtractClosed

partial def Decl.extractClosed (decl : Decl) (sccDecls : Array Decl) : CompilerM (Array Decl) := do
  let ⟨decl, s⟩ ← ExtractClosed.visitDecl decl |>.run { baseName := decl.name, sccDecls } |>.run {}
  return s.decls.push decl

def extractClosed : Pass where
  phase := .mono
  name := `extractClosed
  run := fun decls => do
    if (← getConfig).extractClosed then
      decls.foldlM (init := #[]) fun newDecls decl =>
        return newDecls ++ (← decl.extractClosed decls)
    else
      return decls

builtin_initialize registerTraceClass `Compiler.extractClosed (inherited := true)

end Lean.Compiler.LCNF
