use crate::error::{Error, Result};
use crate::{pic, util};
use std::{mem, slice};

pub struct Patcher {
  patch_area: &'static mut [u8],
  /* original_prolog: Vec<u8>,
   * detour_prolog: Vec<u8>, */
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
    Ok(Patcher { patch_area })
  }

  /// Returns the target's patch area.
  pub fn area(&self) -> &[u8] {
    self.patch_area
  }

  /// Either patches or unpatches the function.
  pub unsafe fn toggle(&mut self, enable: bool) {
    unimplemented!()
  }
}
