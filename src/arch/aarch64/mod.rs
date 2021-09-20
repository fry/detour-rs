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
  type CRet = unsafe extern "C" fn() -> i32;

  /// Detours a C function returning an integer, and asserts its return value.
  #[inline(never)]
  unsafe fn detour_test(target: CRet, result: i32) {
    dbg!(ret10 as *const ());
    dbg!(target as *const ());
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
    unsafe extern "C" fn branch_ret5() -> i32 {
      asm!(
        "adr x16, label",
        "nop",
        "nop",
        "mov x0, #5",
        "ret",
        "label:",
        options(noreturn)
      )
    }

    unsafe { detour_test(mem::transmute(branch_ret5 as usize), 5) }
  }

  /// Default detour target.
  unsafe extern "C" fn ret10() -> i32 {
    10
  }
}
