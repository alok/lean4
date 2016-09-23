-- Copyright (c) 2014 Microsoft Corporation. All rights reserved.
-- Released under Apache 2.0 license as described in the file LICENSE.
-- Author: Leonardo de Moura
prelude
import init.datatypes

namespace bool
  attribute [inline]
  def {u} cond {A : Type u} : bool → A → A → A
  | tt a b := a
  | ff a b := b

  attribute [inline]
  def bor : bool → bool → bool
  | tt _  := tt
  | ff tt := tt
  | ff ff := ff

  attribute [inline]
  def band : bool → bool → bool
  | ff _  := ff
  | tt ff := ff
  | tt tt := tt

  attribute [inline]
  def bnot : bool → bool
  | tt := ff
  | ff := tt
end bool

notation a || b := bool.bor a b
notation a && b := bool.band a b
