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
    unsafe extern "C" fn branch_ret5() -> usize {
      asm!(
        "adr x0, label",
        "nop",
        "nop",
        "ret",
        "label:",
        options(noreturn)
      )
    }

    let label_addr = branch_ret5 as usize + 16;
    unsafe { detour_test(mem::transmute(branch_ret5 as usize), label_addr) }
  }

  /// Default detour target.
  unsafe extern "C" fn ret10() -> usize {
    10
  }
}
