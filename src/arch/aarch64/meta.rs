use super::thunk;
use crate::{error::Result, pic};
use bad64::{Imm, Op, Operand, Reg};

/// The furthest distance between a target and its detour (2 GiB).
pub const DETOUR_RANGE: usize = 0x8000_0000;
pub const PAGE_SIZE: usize = 0x1000;
pub const ALIGNMENT: usize = 8;
pub const CONDITIONAL_OPS: &[bad64::Op] = &[
  Op::B_AL,
  Op::B_CS,
  Op::B_EQ,
  Op::B_GE,
  Op::B_GT,
  Op::B_CC,
  Op::B_HI,
  Op::B_LE,
  Op::B_LS,
  Op::B_LT,
  Op::B_MI,
  Op::B_NE,
  Op::B_NV,
  Op::B_PL,
  Op::B_VC,
  Op::B_VS,
];

/// Returns the preferred prolog size for the target.
pub fn prolog_margin(_target: *const ()) -> usize {
  12
}

/// Creates a relay containing the detour address to be loaded by the three
/// instruction indirect jump
pub fn relay_builder(_target: *const (), detour: *const ()) -> Result<Option<pic::CodeEmitter>> {
  let mut emitter = pic::CodeEmitter::new();
  emitter.add_thunk(Box::new(thunk::thunk_dynasm!(
    ; .qword detour as _
  )));
  Ok(Some(emitter))
}

fn imm_to_signed(imm: &Imm) -> i64 {
  match imm {
    Imm::Unsigned(a) => *a as i64,
    Imm::Signed(a) => *a,
  }
}

pub unsafe fn skip_jmps(target: *const ()) -> *const () {
  let code = std::slice::from_raw_parts_mut(target as *mut u8, 12);
  skip_import_jmp(code, target as u64)
    .map(|p| p as *const ())
    .unwrap_or(target)
}

/// Skip potential jumps to import functions
fn skip_import_jmp(code: &[u8], address: u64) -> Option<u64> {
  let instructions: Vec<_> = bad64::disasm(code, address).collect();

  // Two options of indirect jumps:
  // 1) Import address table
  // adrp  x16, IAT
  // ldr   x16, [x16, IAT]
  // br    x16
  // 2) Direct branch
  // b FUNC
  let instruction = instructions.get(0).and_then(|r| r.as_ref().ok())?;
  let page = match instruction.operands() {
    // ADRP
    [Operand::Reg {
      reg: Reg::X16,
      arrspec: None,
    }, Operand::Label(a)]
      if instruction.op() == Op::ADRP =>
    {
      a
    },
    // Direct branch
    [Operand::Label(a)] if instruction.op() == Op::B => return Some(imm_to_signed(a) as u64),
    _ => return None,
  };

  let instruction = instructions.get(1).and_then(|r| r.as_ref().ok())?;
  let page_offset = match instruction.operands() {
    [Operand::Reg { reg: Reg::X16, .. }, Operand::MemOffset {
      reg: Reg::X16,
      offset: a,
      ..
    }] if instruction.op() == Op::LDR => a,
    _ => return None,
  };

  let instruction = instructions.get(2).and_then(|r| r.as_ref().ok())?;
  if instruction.op() != Op::BR {
    return None;
  }

  let br_addr = imm_to_signed(page) + imm_to_signed(page_offset);

  Some(br_addr as u64)
}

pub unsafe fn clear_instruction_cache(code: &[u8]) {
  // invalidate instruction cache
  extern "C" {
    fn __clear_cache(start: *const u8, end: *const u8);
  }
  __clear_cache(code.as_ptr(), code.as_ptr().add(code.len()));
}

#[cfg(test)]
mod tests {
  use super::*;

  // #[test]
  // fn test_find_jmp() {
  //   #[naked]
  //   unsafe extern "C" fn import_func() {
  //     asm!(
  //       "adrp x16, label",
  //       "ldr x16, [x16, :lo12:label]",
  //       "br x16",
  //       "nop",
  //       "label:",
  //       "nop",
  //       options(noreturn)
  //     )
  //   }

  //   let import_func_addr = import_func as *const () as u64;
  //   let mem = unsafe { std::slice::from_raw_parts(import_func as *const u8,
  // 12) };   let real_import_addr = skip_import_jmp(mem, import_func_addr);
  //   // Label is 4 instructions from the start of the function
  //   assert_eq!(real_import_addr, Some(import_func_addr + 4 * 4));
  // }

  // #[test]
  // fn test_find_jmp2() {
  //   let func = [
  //     0x50u8, 0x3F, 0x00, 0xB0, 0x10, 0x7A, 0x41, 0xF9, 0x00, 0x02, 0x1F,
  // 0xD6,   ];

  //   let mem = unsafe { std::slice::from_raw_parts(&func as *const u8, 12) };

  //   let import_addr = skip_import_jmp(mem, 0x101B93A78);

  //   assert_eq!(import_addr, Some(0x10237C2F0));
  // }
}
