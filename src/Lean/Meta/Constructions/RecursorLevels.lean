/-
Copyright (c) 2025 Lean FRO, LLC. All rights reserved.
Released under Apache 2.0 license as described in the file LICENSE.
Authors: OpenAI Codex
-/

module

prelude
public import Lean.Meta.Basic

namespace Lean.Meta

structure RecursorLevels where
  extra      : Nat
  elimLevel  : Level
  elimHLevel : Level
  indLevels  : List Level
  allLevels  : List Level

def getRecursorLevels (recLevelParams indLevelParams : List Name) (indHLevel : Level) : RecursorLevels :=
  let allLevels := recLevelParams.map mkLevelParam
  let indLevelCount := indLevelParams.length
  let extra := allLevels.length - indLevelCount
  let indLevels := allLevels.drop extra
  let elimLevel := if h : extra > 0 then allLevels[0] else levelZero
  let elimHLevel := if h : extra > 1 then allLevels[1] else indHLevel
  { extra, elimLevel, elimHLevel, indLevels, allLevels }

end Lean.Meta
