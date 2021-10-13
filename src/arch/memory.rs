use crate::{alloc, arch, error::Result, pic};
use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
  /// Shared allocator for all detours.
  pub static ref POOL: Mutex<alloc::ThreadAllocator> = {
    // Use a range of +/- 2 GB for seeking a memory block
    Mutex::new(alloc::ThreadAllocator::new(arch::meta::DETOUR_RANGE))
  };
}

/// Allocates PIC code at the specified address.
pub fn allocate_pic(
  pool: &mut alloc::ThreadAllocator,
  emitter: &pic::CodeEmitter,
  origin: *const (),
) -> Result<alloc::ExecutableMemory> {
  // Ensure alignment
  let size = if emitter.len() % arch::meta::ALIGNMENT != 0 {
    (emitter.len() / arch::meta::ALIGNMENT + 1) * arch::meta::ALIGNMENT
  } else {
    emitter.len()
  };
  // Allocate memory close to the origin
  log::debug!("allocating {} bytes close to {:?}", size, origin);
  let mut memory = pool.allocate(origin, size)?;
  memory.modify(|data| {
    // Generate code for the obtained address
    let code = emitter.emit(data.as_ptr() as *const _);
    data[..code.len()].copy_from_slice(code.as_slice());
  })?;
  Ok(memory)
}
