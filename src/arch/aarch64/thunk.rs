use super::meta;
use crate::error::{Error, Result};
use crate::pic::{self, FixedThunk};
use bad64::{Imm, Instruction, Op, Operand, Reg};
use dynasmrt::{dynasm, DynasmApi, DynasmLabelApi};
use generic_array::{typenum, GenericArray};
use std::ops::Deref;

macro_rules! thunk_dynasm {
    ($($t:tt)*) => {{
      use std::ops::{Deref};
      use dynasmrt::{dynasm, DynasmApi};
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

pub fn gen_adr(instruction: &Instruction) -> Result<Box<dyn pic::Thunkable>> {
  assert_eq!(instruction.op(), Op::ADR);
  Ok(match instruction.operands() {
    [Operand::Reg { reg, arrspec: None }, Operand::Label(Imm::Unsigned(target))] => {
      let target = *target as usize;
      let reg = *reg;
      Box::new(unsafe {
        pic::UnsafeThunk::new(
          move |dest| {
            let delta = target as isize - dest as isize;
            let delta_page =
              (target / meta::PAGE_SIZE) as isize - (dest / meta::PAGE_SIZE) as isize;
            let page = (target & !0xfff) as isize - (dest & !0xfff) as isize;

            let max_range = bit_range(20);
            if max_range.contains(&delta) {
              thunk_dynasm!(
                ; adr X(reg_no(reg).unwrap()), delta
              )
            } else if max_range.contains(&delta_page) {
              thunk_dynasm!(
                  ; adrp X(reg_no(reg).unwrap()), page
                  ; add X(reg_no(reg).unwrap()), X(reg_no(reg).unwrap()), (target & 0xFFF) as u32
              )
            } else {
              // TODO: handle 32bit register type
              gen_mov_64(reg, target)
            }
          },
          4 * 4,
        )
      })
    },
    _ => unimplemented!(),
  })
}

pub fn gen_adrp(instruction: &Instruction) -> Result<Box<dyn pic::Thunkable>> {
  assert_eq!(instruction.op(), Op::ADRP);
  Ok(match instruction.operands() {
    [Operand::Reg { reg, arrspec: None }, Operand::Label(Imm::Unsigned(target))] => {
      let target = *target as usize;
      let reg = *reg;
      Box::new(unsafe {
        pic::UnsafeThunk::new(
          move |dest| {
            let delta_page =
              (target / meta::PAGE_SIZE) as isize - (dest / meta::PAGE_SIZE) as isize;
            let page = (target & !0xfff) as isize - (dest & !0xfff) as isize;
            let max_range = bit_range(20);
            if max_range.contains(&page) {
              thunk_dynasm!(
                  ; adrp X(reg_no(reg).unwrap()), delta_page
              )
            } else {
              // TODO: handle 32bit register type
              gen_mov_64(reg, target)
            }
          },
          4 * 4,
        )
      })
    },
    _ => unimplemented!(),
  })
}

pub fn gen_ldr_literal(instruction: &Instruction) -> Result<Box<dyn pic::Thunkable>> {
  assert_eq!(instruction.op(), Op::LDR);
  Ok(match instruction.operands() {
    [Operand::Reg { reg, arrspec: None }, Operand::Label(Imm::Unsigned(target))] => {
      let target = *target as usize;
      let reg = *reg;
      Box::new(unsafe {
        pic::UnsafeThunk::new(
          move |dest| {
            let delta = target as isize - dest as isize;
            let max_range = bit_range(21);
            if max_range.contains(&delta) {
              // TODO: support 32 bit target register
              // TODO: support fp neon target register
              thunk_dynasm!(
                  ; ldr X(reg_no(reg).unwrap()), delta
              )
            } else {
              // Store address in temporary register, then load
              // TODO: support 32 bit target register
              let reg = reg_no(reg).unwrap();
              thunk_dynasm!(
                  ; movk X(17), (target & 0xFFFF) as u32, LSL 0
                  ; movk X(17), ((target >> 16) & 0xFFFF) as u32, LSL 16
                  ; movk X(17), ((target >> 32) & 0xFFFF) as u32, LSL 32
                  ; movk X(17), ((target >> 48) & 0xFFFF) as u32, LSL 48
                  ; ldr X(reg), [X(17)]
              )
            }
          },
          5 * 4,
        )
      })
    },
    _ => unimplemented!(),
  })
}

pub fn gen_mov_64(reg: Reg, value: usize) -> Vec<u8> {
  let reg = reg_no(reg).unwrap();
  thunk_dynasm!(
      ; movk X(reg), (value & 0xFFFF) as u32, LSL 0
      ; movk X(reg), ((value >> 16) & 0xFFFF) as u32, LSL 16
      ; movk X(reg), ((value >> 32) & 0xFFFF) as u32, LSL 32
      ; movk X(reg), ((value >> 48) & 0xFFFF) as u32, LSL 48
  )
}

#[inline]
fn bit_range(bits: u8) -> std::ops::Range<isize> {
  let max_val = 1isize << bits;
  -max_val..max_val
}

fn reg_no(reg: Reg) -> Option<u32> {
  Some(match reg {
    Reg::X0 => 0,
    Reg::X1 => 1,
    Reg::X2 => 2,
    Reg::X3 => 3,
    Reg::X4 => 4,
    Reg::X5 => 5,
    Reg::X6 => 6,
    Reg::X7 => 7,
    Reg::X8 => 8,
    Reg::X9 => 9,
    Reg::X10 => 10,
    Reg::X11 => 11,
    Reg::X12 => 12,
    Reg::X13 => 13,
    Reg::X14 => 14,
    Reg::X15 => 15,
    Reg::X16 => 16,
    Reg::X17 => 17,
    Reg::X18 => 18,
    Reg::X19 => 19,
    Reg::X20 => 20,
    Reg::X21 => 21,
    Reg::X22 => 22,
    Reg::X23 => 23,
    Reg::X24 => 24,
    Reg::X25 => 25,
    Reg::X26 => 26,
    Reg::X27 => 27,
    Reg::X28 => 28,
    Reg::X29 => 29,
    Reg::X30 => 30,
    _ => return None,
  })
}
