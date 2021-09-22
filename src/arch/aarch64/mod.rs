pub use self::patcher::Patcher;
pub use self::trampoline::Trampoline;

pub mod meta;
mod patcher;
mod thunk;
mod trampoline;

#[cfg(all(feature = "nightly", test))]
mod tests {
  use super::*;
  use crate::*;
  use std::mem;

  /// Default test case function definition.
  type CRet = unsafe extern "C" fn() -> usize;

  /// Detours a C function returning an integer, and asserts its return value.
  #[inline(never)]
  unsafe fn detour_test(target: CRet, result: usize) {
    let hook = RawDetour::new(target as *const (), ret10 as *const ()).unwrap();
    assert_eq!(target(), result);
    hook.enable().unwrap();
    {
      assert_eq!(target(), 10);
      let original: CRet = mem::transmute(hook.trampoline());
      assert_eq!(original(), result);
    }
    hook.disable().unwrap();
    assert_eq!(target(), result);
  }

  #[test]
  fn detour_adr() {
    #[naked]
    unsafe extern "C" fn branch_ret() -> usize {
      asm!(
        "adr x0, label",
        "nop",
        "nop",
        "ret",
        "label:",
        options(noreturn)
      )
    }

    let label_addr = branch_ret as usize + 16;
    unsafe { detour_test(mem::transmute(branch_ret as usize), label_addr) }
  }

  #[test]
  fn detour_adrp() {
    #[naked]
    unsafe extern "C" fn branch_ret() -> usize {
      asm!(
        "adrp x0, .Llabel",
        "nop",
        "nop",
        "ret",
        ".Llabel:",
        options(noreturn)
      )
    }

    let label_addr = (branch_ret as usize + 16) & !0xFFF;
    unsafe { detour_test(mem::transmute(branch_ret as usize), label_addr) }
  }

  #[test]
  fn detour_ldr() {
    #[naked]
    unsafe extern "C" fn branch_ret() -> usize {
      asm!(
        "ldr x0, 2f",
        "nop",
        "nop",
        "ret",
        "2:",
        "nop",
        "nop",
        options(noreturn)
      )
    }

    // two NOPs
    let value = 0xD503201FD503201F;
    unsafe { detour_test(mem::transmute(branch_ret as usize), value) }
  }

  /// Default detour target.
  unsafe extern "C" fn ret10() -> usize {
    10
  }
}
