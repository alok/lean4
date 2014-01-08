import macros

universe M ≥ 512
universe U ≥ M+512

variable Bool : Type
-- The following builtin declarations can be removed as soon as Lean supports inductive datatypes and match expressions
builtin true : Bool
builtin false : Bool
builtin if {A : (Type U)} : Bool → A → A → A

definition TypeU := (Type U)
definition TypeM := (Type M)

definition not (a : Bool) : Bool
:= a → false

notation 40 ¬ _ : not

definition or (a b : Bool) : Bool
:= ¬ a → b

infixr 30 || : or
infixr 30 \/ : or
infixr 30 ∨  : or

definition and (a b : Bool) : Bool
:= ¬ (a → ¬ b)

definition implies (a b : Bool) : Bool
:= a → b

infixr 35 && : and
infixr 35 /\ : and
infixr 35 ∧  : and

definition Exists (A : TypeU) (P : A → Bool) : Bool
:= ¬ (∀ x : A, ¬ (P x))

definition eq {A : TypeU} (a b : A) : Bool
:= a == b

infix 50 = : eq

definition neq {A : TypeU} (a b : A) : Bool
:= ¬ (a == b)

infix 50 ≠ : neq

axiom case (P : Bool → Bool) (H1 : P true) (H2 : P false) (a : Bool) : P a

axiom refl {A : TypeU} (a : A) : a == a

axiom subst {A : TypeU} {a b : A} {P : A → Bool} (H1 : P a) (H2 : a == b) : P b

definition substp {A : TypeU} {a b : A} (P : A → Bool) (H1 : P a) (H2 : a == b) : P b
:= subst H1 H2

axiom eta {A : TypeU} {B : A → TypeU} (f : ∀ x : A, B x) : (λ x : A, f x) == f

axiom iff::intro {a b : Bool} (H1 : a → b) (H2 : b → a) : a == b

axiom abst {A : TypeU} {B : A → TypeU} {f g : ∀ x : A, B x} (H : ∀ x : A, f x == g x) : f == g

axiom abstpi {A : TypeU} {B C : A → TypeU} (H : ∀ x : A, B x == C x) : (∀ x : A, B x) == (∀ x : A, C x)

axiom hsymm {A B : TypeU} {a : A} {b : B} (H : a == b) : b == a

axiom htrans {A B C : TypeU} {a : A} {b : B} {c : C} (H1 : a == b) (H2 : b == c) : a == c

theorem trivial : true
:= refl true

theorem em (a : Bool) : a ∨ ¬ a
:= case (λ x, x ∨ ¬ x) trivial trivial a

theorem false::elim (a : Bool) (H : false) : a
:= case (λ x, x) trivial H a

theorem absurd {a : Bool} (H1 : a) (H2 : ¬ a) : false
:= H2 H1

theorem eqmp {a b : Bool} (H1 : a == b) (H2 : a) : b
:= subst H2 H1

infixl 100 <| : eqmp
infixl 100 ◂  : eqmp

-- assume is a 'macro' that expands into a discharge

theorem imp::trans {a b c : Bool} (H1 : a → b) (H2 : b → c) : a → c
:= λ Ha, H2 (H1 Ha)

theorem imp::eq::trans {a b c : Bool} (H1 : a → b) (H2 : b == c) : a → c
:= λ Ha, H2 ◂ (H1 Ha)

theorem eq::imp::trans {a b c : Bool} (H1 : a == b) (H2 : b → c) : a → c
:= λ Ha, H2 (H1 ◂ Ha)

theorem not::not::eq (a : Bool) : (¬ ¬ a) == a
:= case (λ x, (¬ ¬ x) == x) trivial trivial a

theorem not::not::elim {a : Bool} (H : ¬ ¬ a) : a
:= (not::not::eq a) ◂ H

theorem mt {a b : Bool} (H1 : a → b) (H2 : ¬ b) : ¬ a
:= λ Ha, absurd (H1 Ha) H2

theorem contrapos {a b : Bool} (H : a → b) : ¬ b → ¬ a
:= λ Hnb : ¬ b, mt H Hnb

theorem absurd::elim {a : Bool} (b : Bool) (H1 : a) (H2 : ¬ a) : b
:= false::elim b (absurd H1 H2)

theorem not::imp::eliml {a b : Bool} (Hnab : ¬ (a → b)) : a
:= not::not::elim
      (have ¬ ¬ a :
       λ Hna : ¬ a, absurd (have a → b     : λ Ha : a, absurd::elim b Ha Hna)
                           Hnab)

theorem not::imp::elimr {a b : Bool} (H : ¬ (a → b)) : ¬ b
:= λ Hb : b, absurd (have a → b : λ Ha : a, Hb)
                    (have ¬ (a → b) : H)

theorem resolve1 {a b : Bool} (H1 : a ∨ b) (H2 : ¬ a) : b
:= H1 H2

-- Remark: conjunction is defined as ¬ (a → ¬ b) in Lean

theorem and::intro {a b : Bool} (H1 : a) (H2 : b) : a ∧ b
:= λ H : a → ¬ b, absurd H2 (H H1)

theorem and::eliml {a b : Bool} (H : a ∧ b) : a
:= not::imp::eliml H

theorem and::elimr {a b : Bool} (H : a ∧ b) : b
:= not::not::elim (not::imp::elimr H)

-- Remark: disjunction is defined as ¬ a → b in Lean

theorem or::introl {a : Bool} (H : a) (b : Bool) : a ∨ b
:= λ H1 : ¬ a, absurd::elim b H H1

theorem or::intror {b : Bool} (a : Bool) (H : b) : a ∨ b
:= λ H1 : ¬ a, H

theorem or::elim {a b c : Bool} (H1 : a ∨ b) (H2 : a → c) (H3 : b → c) : c
:= not::not::elim
     (λ H : ¬ c,
        absurd (have c : H3 (have b : resolve1 H1 (have ¬ a : (mt (λ Ha : a, H2 Ha) H))))
               H)

theorem refute {a : Bool} (H : ¬ a → false) : a
:= or::elim (em a) (λ H1 : a, H1) (λ H1 : ¬ a, false::elim a (H H1))

theorem symm {A : TypeU} {a b : A} (H : a == b) : b == a
:= subst (refl a) H

theorem trans {A : TypeU} {a b c : A} (H1 : a == b) (H2 : b == c) : a == c
:= subst H1 H2

infixl 100 ⋈ : trans

theorem ne::symm {A : TypeU} {a b : A} (H : a ≠ b) : b ≠ a
:= λ H1 : b = a, H (symm H1)

theorem eq::ne::trans {A : TypeU} {a b c : A} (H1 : a = b) (H2 : b ≠ c) : a ≠ c
:= subst H2 (symm H1)

theorem ne::eq::trans {A : TypeU} {a b c : A} (H1 : a ≠ b) (H2 : b = c) : a ≠ c
:= subst H1 H2

theorem eqt::elim {a : Bool} (H : a == true) : a
:= (symm H) ◂ trivial

theorem eqt::intro {a : Bool} (H : a) : a == true
:= iff::intro (λ H1 : a, trivial)
              (λ H2 : true, H)

theorem congr1 {A : TypeU} {B : A → TypeU} {f g : ∀ x : A, B x} (a : A) (H : f == g) : f a == g a
:= substp (fun h : (∀ x : A, B x), f a == h a) (refl (f a)) H

-- Remark: we must use heterogeneous equality in the following theorem because the types of (f a) and (f b)
-- are not "definitionally equal" They are (B a) and (B b)
-- They are provably equal, we just have to apply Congr1

theorem congr2 {A : TypeU} {B : A → TypeU} {a b : A} (f : ∀ x : A, B x) (H : a == b) : f a == f b
:= substp (fun x : A, f a == f x) (refl (f a)) H

-- Remark: like the previous theorem we use heterogeneous equality We cannot use Trans theorem
-- because the types are not definitionally equal

theorem congr {A : TypeU} {B : A → TypeU} {f g : ∀ x : A, B x} {a b : A} (H1 : f == g) (H2 : a == b) : f a == g b
:= htrans (congr2 f H2) (congr1 b H1)

-- Remark: the existential is defined as (¬ (forall x : A, ¬ P x))

theorem exists::elim {A : TypeU} {P : A → Bool} {B : Bool} (H1 : Exists A P) (H2 : ∀ (a : A) (H : P a), B) : B
:= refute (λ R : ¬ B,
             absurd (λ a : A, mt (λ H : P a, H2 a H) R)
                    H1)

theorem exists::intro {A : TypeU} {P : A → Bool} (a : A) (H : P a) : Exists A P
:= λ H1 : (∀ x : A, ¬ P x),
      absurd H (H1 a)

-- At this point, we have proved the theorems we need using the
-- definitions of forall, exists, and, or, =>, not  We mark (some of)
-- them as opaque Opaque definitions improve performance, and
-- effectiveness of Lean's elaborator

theorem or::comm (a b : Bool) : (a ∨ b) == (b ∨ a)
:= iff::intro (λ H, or::elim H (λ H1, or::intror b H1) (λ H2, or::introl H2 a))
              (λ H, or::elim H (λ H1, or::intror a H1) (λ H2, or::introl H2 b))

theorem or::assoc (a b c : Bool) : ((a ∨ b) ∨ c) == (a ∨ (b ∨ c))
:= iff::intro (λ H : (a ∨ b) ∨ c,
                      or::elim H (λ H1 : a ∨ b, or::elim H1 (λ Ha : a, or::introl Ha (b ∨ c)) (λ Hb : b, or::intror a (or::introl Hb c)))
                                  (λ Hc : c, or::intror a (or::intror b Hc)))
              (λ H : a ∨ (b ∨ c),
                      or::elim H (λ Ha : a, (or::introl (or::introl Ha b) c))
                                  (λ H1 : b ∨ c, or::elim H1 (λ Hb : b, or::introl (or::intror a Hb) c)
                                                              (λ Hc : c, or::intror (a ∨ b) Hc)))

theorem or::id (a : Bool) : (a ∨ a) == a
:= iff::intro (λ H, or::elim H (λ H1, H1) (λ H2, H2))
              (λ H, or::introl H a)

theorem or::falsel (a : Bool) : (a ∨ false) == a
:= iff::intro (λ H, or::elim H (λ H1, H1) (λ H2, false::elim a H2))
              (λ H, or::introl H false)

theorem or::falser (a : Bool) : (false ∨ a) == a
:= (or::comm false a) ⋈ (or::falsel a)

theorem or::truel (a : Bool) : (true ∨ a) == true
:= eqt::intro (case (λ x : Bool, true ∨ x) trivial trivial a)

theorem or::truer (a : Bool) : (a ∨ true) == true
:= (or::comm a true) ⋈ (or::truel a)

theorem or::tauto (a : Bool) : (a ∨ ¬ a) == true
:= eqt::intro (em a)

theorem and::comm (a b : Bool) : (a ∧ b) == (b ∧ a)
:= iff::intro (λ H, and::intro (and::elimr H) (and::eliml H))
              (λ H, and::intro (and::elimr H) (and::eliml H))

theorem and::id (a : Bool) : (a ∧ a) == a
:= iff::intro (λ H, and::eliml H)
              (λ H, and::intro H H)

theorem and::assoc (a b c : Bool) : ((a ∧ b) ∧ c) == (a ∧ (b ∧ c))
:= iff::intro (λ H, and::intro (and::eliml (and::eliml H)) (and::intro (and::elimr (and::eliml H)) (and::elimr H)))
              (λ H, and::intro (and::intro (and::eliml H) (and::eliml (and::elimr H))) (and::elimr (and::elimr H)))

theorem and::truer (a : Bool) : (a ∧ true) == a
:= iff::intro (λ H : a ∧ true,  and::eliml H)
              (λ H : a,         and::intro H trivial)

theorem and::truel (a : Bool) : (true ∧ a) == a
:= trans (and::comm true a) (and::truer a)

theorem and::falsel (a : Bool) : (a ∧ false) == false
:= iff::intro (λ H, and::elimr H)
              (λ H, false::elim (a ∧ false) H)

theorem and::falser (a : Bool) : (false ∧ a) == false
:= (and::comm false a) ⋈ (and::falsel a)

theorem and::absurd (a : Bool) : (a ∧ ¬ a) == false
:= iff::intro (λ H, absurd (and::eliml H) (and::elimr H))
              (λ H, false::elim (a ∧ ¬ a) H)

theorem not::true : (¬ true) == false
:= trivial

theorem not::false : (¬ false) == true
:= trivial

theorem not::and (a b : Bool) : (¬ (a ∧ b)) == (¬ a ∨ ¬ b)
:= case (λ x, (¬ (x ∧ b)) == (¬ x ∨ ¬ b))
        (case (λ y, (¬ (true ∧ y)) == (¬ true ∨ ¬ y))   trivial trivial b)
        (case (λ y, (¬ (false ∧ y)) == (¬ false ∨ ¬ y)) trivial trivial b)
        a

theorem not::and::elim {a b : Bool} (H : ¬ (a ∧ b)) : ¬ a ∨ ¬ b
:= (not::and a b) ◂ H

theorem not::or (a b : Bool) : (¬ (a ∨ b)) == (¬ a ∧ ¬ b)
:= case (λ x, (¬ (x ∨ b)) == (¬ x ∧ ¬ b))
        (case (λ y, (¬ (true ∨ y)) == (¬ true ∧ ¬ y))   trivial trivial b)
        (case (λ y, (¬ (false ∨ y)) == (¬ false ∧ ¬ y)) trivial trivial b)
        a

theorem not::or::elim {a b : Bool} (H : ¬ (a ∨ b)) : ¬ a ∧ ¬ b
:= (not::or a b) ◂ H

theorem not::iff (a b : Bool) : (¬ (a == b)) == ((¬ a) == b)
:= case (λ x, (¬ (x == b)) == ((¬ x) == b))
        (case (λ y, (¬ (true == y)) == ((¬ true) == y)) trivial trivial b)
        (case (λ y, (¬ (false == y)) == ((¬ false) == y)) trivial trivial b)
        a

theorem not::iff::elim {a b : Bool} (H : ¬ (a == b)) : (¬ a) == b
:= (not::iff a b) ◂ H

theorem not::implies (a b : Bool) : (¬ (a → b)) == (a ∧ ¬ b)
:= case (λ x, (¬ (x → b)) == (x ∧ ¬ b))
        (case (λ y, (¬ (true → y)) == (true ∧ ¬ y)) trivial trivial b)
        (case (λ y, (¬ (false → y)) == (false ∧ ¬ y)) trivial trivial b)
        a

theorem not::implies::elim {a b : Bool} (H : ¬ (a → b)) : a ∧ ¬ b
:= (not::implies a b) ◂ H

theorem not::congr {a b : Bool} (H : a == b) : (¬ a) == (¬ b)
:= congr2 not H

theorem eq::exists::intro {A : (Type U)} {P Q : A → Bool} (H : ∀ x : A, P x == Q x) : (∃ x : A, P x) == (∃ x : A, Q x)
:= congr2 (Exists A) (abst H)

theorem not::forall (A : (Type U)) (P : A → Bool) : (¬ (∀ x : A, P x)) == (∃ x : A, ¬ P x)
:= let
       l1 : ∀ x : A, P x == ¬ ¬ P x                    := λ x : A, symm (not::not::eq (P x)),
       l2 : (∀ x : A, P x) == (∀ x : A, ¬ ¬ P x)       := abstpi l1,
       s1 : (¬ ∀ x : A, P x) == (¬ ∀ x : A, ¬ ¬ P x)   := not::congr l2,
       s2 : (¬ ∀ x : A, ¬ ¬ P x) == (∃ x : A, ¬ P x)   := refl (∃ x : A, ¬ P x)
   in trans s1 s2

theorem not::forall::elim {A : (Type U)} {P : A → Bool} (H : ¬ (∀ x : A, P x)) : ∃ x : A, ¬ P x
:= (not::forall A P) ◂ H

theorem not::exists (A : (Type U)) (P : A → Bool) : (¬ ∃ x : A, P x) == (∀ x : A, ¬ P x)
:= calc (¬ ∃ x : A, P x) = (¬ ¬ ∀ x : A, ¬ P x) : refl (¬ ∃ x : A, P x)
                     ... = (∀ x : A, ¬ P x)     : not::not::eq (∀ x : A, ¬ P x)

theorem not::exists::elim {A : (Type U)} {P : A → Bool} (H : ¬ ∃ x : A, P x) : ∀ x : A, ¬ P x
:= (not::exists A P) ◂ H

theorem exists::unfold1 {A : TypeU} {P : A → Bool} (a : A) (H : ∃ x : A, P x) : P a ∨ (∃ x : A, x ≠ a ∧ P x)
:= exists::elim H
     (λ (w : A) (H1 : P w),
        or::elim (em (w = a))
          (λ Heq : w = a, or::introl (subst H1 Heq) (∃ x : A, x ≠ a ∧ P x))
          (λ Hne : w ≠ a, or::intror (P a) (exists::intro w (and::intro Hne H1))))

theorem exists::unfold2 {A : TypeU} {P : A → Bool} (a : A) (H : P a ∨ (∃ x : A, x ≠ a ∧ P x)) : ∃ x : A, P x
:= or::elim H
      (λ H1 : P a, exists::intro a H1)
      (λ H2 : (∃ x : A, x ≠ a ∧ P x),
          exists::elim H2
               (λ (w : A) (Hw : w ≠ a ∧ P w),
                  exists::intro w (and::elimr Hw)))

theorem exists::unfold {A : TypeU} (P : A → Bool) (a : A) : (∃ x : A, P x) = (P a ∨ (∃ x : A, x ≠ a ∧ P x))
:= iff::intro (λ H : (∃ x : A, P x), exists::unfold1 a H)
              (λ H : (P a ∨ (∃ x : A, x ≠ a ∧ P x)), exists::unfold2 a H)

set::opaque exists  true
set::opaque not     true
set::opaque or      true
set::opaque and     true
