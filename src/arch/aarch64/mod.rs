pub use self::patcher::Patcher;
pub use self::trampoline::Trampoline;

pub mod meta;
mod patcher;
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
  unsafe fn detour_test(target: CRet, result: i32) -> Result<()> {
    let hook = RawDetour::new(target as *const (), ret10 as *const ())?;
let insts: Vec<_> = bad64::disasm(&hook.trampoline(), memory.as_ptr() as u64).collect();
    dbg!()
    assert_eq!(target(), result);
    hook.enable()?;
    {
      assert_eq!(target(), 10);
      let original: CRet = mem::transmute(hook.trampoline());
      assert_eq!(original(), result);
    }
    hook.disable()?;
    assert_eq!(target(), result);
    Ok(())
  }

  #[test]
  fn detour_adr() -> Result<()> {
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
