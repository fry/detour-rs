use super::thunk;
use crate::{error::Result, pic};
use std::mem;
use bad64::{Operand, Op, Reg};

/// The furthest distance between a target and its detour (2 GiB).
pub const DETOUR_RANGE: usize = 0x8000_0000;

/// Returns the preferred prolog size for the target.
pub fn prolog_margin(_target: *const ()) -> usize {
  // adrp x17, [jmpval]
  // ldr x17, [x17, jmpval]
  // br x17
  12
}

/// Creates a relay; required for destinations further away than 2GB (on x64).
pub fn relay_builder(target: *const (), detour: *const ()) -> Result<Option<pic::CodeEmitter>> {
  panic!()
}

/// Skip potential jumps to import functions
pub fn skip_import_jmp(target: *const ()) -> *const () {
  let mem = std::slice::from_raw_parts(target, 12);
  let instructions: Vec<_> = bad64::disasm(mem, target as u64).collect();

  // adrp  x16, IAT
  // ldr   x16, [x16, IAT]
  // br    x16
  if instructions[0].op() == Op::ADDP && instructions[0].operands()[0] == Operand::Reg { reg: Reg::X16, arrspec: None } {

  }
}

#[cfg(test)]
mod tests {
  #[test]
  fn test_find_jmp() {
    unsafe extern "C" fn import_func() {
      asm!(
        "adrp x16, #1234",
        "ldr x16, [x16, #1234]"
        "br x16"
      )
    }

    skip_import_jmp(options as *const ());
  }
}
