use super::meta;
use super::thunk;
use crate::error::Result;
use crate::pic;
use std::slice;

pub struct Patcher {
  patch_area: &'static mut [u8],
  original_prolog: Vec<u8>,
  detour_prolog: Vec<u8>,
}

impl Patcher {
  /// Creates a new detour patcher for an address.
  ///
  /// # Arguments
  ///
  /// * `target` - An address that should be hooked.
  /// * `detour` - An address that the target should be redirected to.
  /// * `prolog_size` - The available inline space for the hook.
  pub unsafe fn new(target: *const (), detour: *const (), prolog_size: usize) -> Result<Patcher> {
    let patch_area = slice::from_raw_parts_mut(target as *mut u8, prolog_size);

    let original_prolog = patch_area.to_vec();
    Ok(Patcher {
      original_prolog,
      detour_prolog: Self::hook_template(detour).emit(target),
      patch_area,
    })
  }

  /// Returns the target's patch area.
  pub fn area(&self) -> &[u8] {
    self.patch_area
  }

  fn hook_template(detour: *const ()) -> pic::CodeEmitter {
    let mut emitter = pic::CodeEmitter::new();
    emitter.add_thunk(thunk::gen_jmp_indirect(detour as usize));
    emitter
  }

  /// Either patches or unpatches the function.
  pub unsafe fn toggle(&mut self, enable: bool) {
    println!("enable {}", enable);
    // Copy either the detour or the original bytes of the function
    self.patch_area.copy_from_slice(if enable {
      &self.detour_prolog
    } else {
      &self.original_prolog
    });

    meta::clear_instruction_cache(self.patch_area);

    // let  instructions: Vec<_> =
    //   bad64::disasm(self.patch_area, self.patch_area.as_ptr() as
    // u64).collect(); dbg!(&instructions);
  }
}
