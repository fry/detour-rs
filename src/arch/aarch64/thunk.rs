use super::meta;
use crate::pic::{self, FixedThunk};
use dynasmrt::{dynasm, DynasmApi, DynasmLabelApi};
use generic_array::{typenum, GenericArray};
use std::ops::{Deref, DerefMut};

macro_rules! thunk_dynasm {
    ($($t:tt)*) => {{
      use std::ops::{Deref, DerefMut};
      use dynasmrt::{dynasm, DynasmApi, DynasmLabelApi};
      let mut ops = dynasmrt::aarch64::Assembler::new().unwrap();
      dynasm!(ops
        ; .arch aarch64
        $($t)*
      );
      let buf = ops.finalize().unwrap();
      buf.deref().to_vec()
    }}
  }
pub(crate) use thunk_dynasm;

// Generate a branch to an absolute address. Takes 4 + 4 + 8 = 16 instructions
pub fn gen_jmp_immediate(target: usize) -> Box<dyn pic::Thunkable> {
  Box::new(thunk_dynasm!(
    ; ldr x17, >target
    ; br x17
    ; target:
    ; .qword target as _
  ))
}

// Generate a branch to an address loaded from a memory location located
// relative to the instruction address. Takes 4 + 4 + 4 = 12 instructions
pub fn gen_jmp_indirect(detour_value: usize) -> Box<dyn pic::Thunkable> {
  Box::new(FixedThunk::<typenum::U12>::new(move |pc| {
    let page = (detour_value & !0xfff) as isize - (pc & !0xfff) as isize;
    let page_off = (detour_value & 0xfff);
    GenericArray::clone_from_slice(&thunk_dynasm!(
      ; adrp x17, page
      ; ldr x17, [x17, page_off as u32 ]
      ; br x17
    ))
  }))
}
