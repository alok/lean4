use crate::LeanObject;
use crate::object::LeanObj;
use crate::ptr;

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ReducibilityHintsKind {
    Opaque = 0,
    Abbreviation = 1,
    Regular = 2,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DefinitionSafety {
    Unsafe = 0,
    Safe = 1,
    Partial = 2,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DeclarationKind {
    Axiom = 0,
    Definition = 1,
    Theorem = 2,
    Opaque = 3,
    Quot = 4,
    MutualDefinition = 5,
    Inductive = 6,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ConstantInfoKind {
    Axiom = 0,
    Definition = 1,
    Theorem = 2,
    Opaque = 3,
    Quot = 4,
    Inductive = 5,
    Constructor = 6,
    Recursor = 7,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum QuotKind {
    Type = 0,
    Mk = 1,
    Lift = 2,
    Ind = 3,
}

pub struct ConstantVal<'a> {
    pub obj: LeanObj<'a>,
}

impl<'a> ConstantVal<'a> {
    pub fn name(&self) -> *mut LeanObject {
        unsafe { *self.obj.ctor_obj_ptr() }
    }
    pub fn lparams(&self) -> *mut LeanObject {
        unsafe { *self.obj.ctor_obj_ptr().add(1) }
    }
    pub fn type_(&self) -> *mut LeanObject {
        unsafe { *self.obj.ctor_obj_ptr().add(2) }
    }
}

pub struct AxiomVal<'a> {
    pub obj: LeanObj<'a>,
}

impl<'a> AxiomVal<'a> {
    pub fn to_constant_val(&self) -> ConstantVal<'a> {
        unsafe { ConstantVal { obj: LeanObj::new(*self.obj.ctor_obj_ptr()).unwrap() } }
    }
    pub fn is_unsafe(&self) -> bool {
        unsafe { self.obj.ctor_scalar_get_u8(0) != 0 }
    }
}

pub struct DefinitionVal<'a> {
    pub obj: LeanObj<'a>,
}

impl<'a> DefinitionVal<'a> {
    pub fn to_constant_val(&self) -> ConstantVal<'a> {
        unsafe { ConstantVal { obj: LeanObj::new(*self.obj.ctor_obj_ptr()).unwrap() } }
    }
    pub fn value(&self) -> *mut LeanObject {
        unsafe { *self.obj.ctor_obj_ptr().add(1) }
    }
    pub fn hints(&self) -> *mut LeanObject {
        unsafe { *self.obj.ctor_obj_ptr().add(2) }
    }
    pub fn safety(&self) -> DefinitionSafety {
        unsafe { std::mem::transmute(self.obj.ctor_scalar_get_u8(0)) }
    }
}

pub struct TheoremVal<'a> {
    pub obj: LeanObj<'a>,
}

impl<'a> TheoremVal<'a> {
    pub fn to_constant_val(&self) -> ConstantVal<'a> {
        unsafe { ConstantVal { obj: LeanObj::new(*self.obj.ctor_obj_ptr()).unwrap() } }
    }
    pub fn value(&self) -> *mut LeanObject {
        unsafe { *self.obj.ctor_obj_ptr().add(1) }
    }
}

pub struct OpaqueVal<'a> {
    pub obj: LeanObj<'a>,
}

impl<'a> OpaqueVal<'a> {
    pub fn to_constant_val(&self) -> ConstantVal<'a> {
        unsafe { ConstantVal { obj: LeanObj::new(*self.obj.ctor_obj_ptr()).unwrap() } }
    }
    pub fn value(&self) -> *mut LeanObject {
        unsafe { *self.obj.ctor_obj_ptr().add(1) }
    }
    pub fn is_unsafe(&self) -> bool {
        unsafe { self.obj.ctor_scalar_get_u8(0) != 0 }
    }
}

pub struct QuotVal<'a> {
    pub obj: LeanObj<'a>,
}

impl<'a> QuotVal<'a> {
    pub fn to_constant_val(&self) -> ConstantVal<'a> {
        unsafe { ConstantVal { obj: LeanObj::new(*self.obj.ctor_obj_ptr()).unwrap() } }
    }
    pub fn kind(&self) -> QuotKind {
        unsafe { std::mem::transmute(self.obj.ctor_scalar_get_u8(0)) }
    }
}

pub struct InductiveVal<'a> {
    pub obj: LeanObj<'a>,
}

impl<'a> InductiveVal<'a> {
    pub fn to_constant_val(&self) -> ConstantVal<'a> {
        unsafe { ConstantVal { obj: LeanObj::new(*self.obj.ctor_obj_ptr()).unwrap() } }
    }
    pub fn num_params(&self) -> usize {
        unsafe { ptr::unbox_ptr(*self.obj.ctor_obj_ptr().add(1)) }
    }
    pub fn num_indices(&self) -> usize {
        unsafe { ptr::unbox_ptr(*self.obj.ctor_obj_ptr().add(2)) }
    }
    pub fn all(&self) -> *mut LeanObject {
        unsafe { *self.obj.ctor_obj_ptr().add(3) }
    }
    pub fn ctors(&self) -> *mut LeanObject {
        unsafe { *self.obj.ctor_obj_ptr().add(4) }
    }
    pub fn num_nested(&self) -> usize {
        unsafe { ptr::unbox_ptr(*self.obj.ctor_obj_ptr().add(5)) }
    }
    pub fn is_rec(&self) -> bool {
        unsafe { self.obj.ctor_scalar_get_u8(0) != 0 }
    }
    pub fn is_unsafe(&self) -> bool {
        unsafe { self.obj.ctor_scalar_get_u8(1) != 0 }
    }
    pub fn is_reflexive(&self) -> bool {
        unsafe { self.obj.ctor_scalar_get_u8(2) != 0 }
    }
}

pub struct ConstructorVal<'a> {
    pub obj: LeanObj<'a>,
}

impl<'a> ConstructorVal<'a> {
    pub fn to_constant_val(&self) -> ConstantVal<'a> {
        unsafe { ConstantVal { obj: LeanObj::new(*self.obj.ctor_obj_ptr()).unwrap() } }
    }
    pub fn induct(&self) -> *mut LeanObject {
        unsafe { *self.obj.ctor_obj_ptr().add(1) }
    }
    pub fn cidx(&self) -> usize {
        unsafe { ptr::unbox_ptr(*self.obj.ctor_obj_ptr().add(2)) }
    }
    pub fn num_params(&self) -> usize {
        unsafe { ptr::unbox_ptr(*self.obj.ctor_obj_ptr().add(3)) }
    }
    pub fn num_fields(&self) -> usize {
        unsafe { ptr::unbox_ptr(*self.obj.ctor_obj_ptr().add(4)) }
    }
    pub fn is_unsafe(&self) -> bool {
        unsafe { self.obj.ctor_scalar_get_u8(0) != 0 }
    }
}

pub struct RecursorVal<'a> {
    pub obj: LeanObj<'a>,
}

impl<'a> RecursorVal<'a> {
    pub fn to_constant_val(&self) -> ConstantVal<'a> {
        unsafe { ConstantVal { obj: LeanObj::new(*self.obj.ctor_obj_ptr()).unwrap() } }
    }
    pub fn all(&self) -> *mut LeanObject {
        unsafe { *self.obj.ctor_obj_ptr().add(1) }
    }
    pub fn num_params(&self) -> usize {
        unsafe { ptr::unbox_ptr(*self.obj.ctor_obj_ptr().add(2)) }
    }
    pub fn num_indices(&self) -> usize {
        unsafe { ptr::unbox_ptr(*self.obj.ctor_obj_ptr().add(3)) }
    }
    pub fn num_motives(&self) -> usize {
        unsafe { ptr::unbox_ptr(*self.obj.ctor_obj_ptr().add(4)) }
    }
    pub fn num_minors(&self) -> usize {
        unsafe { ptr::unbox_ptr(*self.obj.ctor_obj_ptr().add(5)) }
    }
    pub fn rules(&self) -> *mut LeanObject {
        unsafe { *self.obj.ctor_obj_ptr().add(6) }
    }
    pub fn k(&self) -> bool {
        unsafe { self.obj.ctor_scalar_get_u8(0) != 0 }
    }
    pub fn is_unsafe(&self) -> bool {
        unsafe { self.obj.ctor_scalar_get_u8(1) != 0 }
    }
}

pub struct Declaration<'a> {
    pub obj: LeanObj<'a>,
}

impl<'a> Declaration<'a> {
    pub fn kind(&self) -> DeclarationKind {
        unsafe { std::mem::transmute(crate::layout::header(self.obj.ptr).tag) }
    }

    pub fn to_axiom_val(&self) -> AxiomVal<'a> {
        unsafe { AxiomVal { obj: LeanObj::new(*self.obj.ctor_obj_ptr()).unwrap() } }
    }
    pub fn to_definition_val(&self) -> DefinitionVal<'a> {
        unsafe { DefinitionVal { obj: LeanObj::new(*self.obj.ctor_obj_ptr()).unwrap() } }
    }
    pub fn to_theorem_val(&self) -> TheoremVal<'a> {
        unsafe { TheoremVal { obj: LeanObj::new(*self.obj.ctor_obj_ptr()).unwrap() } }
    }
    pub fn to_opaque_val(&self) -> OpaqueVal<'a> {
        unsafe { OpaqueVal { obj: LeanObj::new(*self.obj.ctor_obj_ptr()).unwrap() } }
    }
    pub fn to_definition_vals(&self) -> *mut LeanObject {
        unsafe { *self.obj.ctor_obj_ptr() }
    }
}

pub struct ConstantInfo<'a> {
    pub obj: LeanObj<'a>,
}

impl<'a> ConstantInfo<'a> {
    pub fn kind(&self) -> ConstantInfoKind {
        unsafe { std::mem::transmute(crate::layout::header(self.obj.ptr).tag) }
    }

    pub fn to_val(&self) -> LeanObj<'a> {
        unsafe { LeanObj::new(*self.obj.ctor_obj_ptr()).unwrap() }
    }

    pub fn to_axiom_val(&self) -> AxiomVal<'a> {
        AxiomVal { obj: self.to_val() }
    }
    pub fn to_definition_val(&self) -> DefinitionVal<'a> {
        DefinitionVal { obj: self.to_val() }
    }
    pub fn to_theorem_val(&self) -> TheoremVal<'a> {
        TheoremVal { obj: self.to_val() }
    }
    pub fn to_opaque_val(&self) -> OpaqueVal<'a> {
        OpaqueVal { obj: self.to_val() }
    }
    pub fn to_inductive_val(&self) -> InductiveVal<'a> {
        InductiveVal { obj: self.to_val() }
    }
    pub fn to_constructor_val(&self) -> ConstructorVal<'a> {
        ConstructorVal { obj: self.to_val() }
    }
    pub fn to_recursor_val(&self) -> RecursorVal<'a> {
        RecursorVal { obj: self.to_val() }
    }
    pub fn to_quot_val(&self) -> QuotVal<'a> {
        QuotVal { obj: self.to_val() }
    }

    pub fn is_unsafe(&self) -> bool {
        match self.kind() {
            ConstantInfoKind::Axiom => self.to_axiom_val().is_unsafe(),
            ConstantInfoKind::Definition => self.to_definition_val().safety() == DefinitionSafety::Unsafe,
            ConstantInfoKind::Theorem => false,
            ConstantInfoKind::Opaque => self.to_opaque_val().is_unsafe(),
            ConstantInfoKind::Quot => false,
            ConstantInfoKind::Inductive => self.to_inductive_val().is_unsafe(),
            ConstantInfoKind::Constructor => self.to_constructor_val().is_unsafe(),
            ConstantInfoKind::Recursor => self.to_recursor_val().is_unsafe(),
        }
    }
}