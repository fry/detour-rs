use super::thunk;
use crate::{error::Result, pic};
use std::mem;

/// The furthest distance between a target and its detour (2 GiB).
pub const DETOUR_RANGE: usize = 0x8000_0000;
pub const ALIGNMENT: usize = 8;

/// Returns the preferred prolog size for the target.
pub fn prolog_margin(_target: *const ()) -> usize {
  mem::size_of::<thunk::x86::JumpRel>()
}

pub unsafe fn skip_jmps(target: *const ()) -> *const () {
  target
}

/// Creates a relay; required for destinations further away than 2GB (on x64).
pub fn relay_builder(target: *const (), detour: *const ()) -> Result<Option<pic::CodeEmitter>> {
  let displacement = (target as isize).wrapping_sub(detour as isize);

  if cfg!(target_arch = "x86_64") && !crate::arch::is_within_range(displacement) {
    let mut emitter = pic::CodeEmitter::new();
    emitter.add_thunk(thunk::jmp(detour as usize));
    Ok(Some(emitter))
  } else {
    Ok(None)
  }
}
