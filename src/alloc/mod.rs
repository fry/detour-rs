use crate::error::Result;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

mod proximity;
mod search;

/// A thread-safe memory pool for allocating chunks close to addresses.
pub struct ThreadAllocator(Arc<Mutex<proximity::ProximityAllocator>>);

// TODO: Decrease use of mutexes
impl ThreadAllocator {
  /// Creates a new proximity memory allocator.
  pub fn new(max_distance: usize) -> Self {
    ThreadAllocator(Arc::new(Mutex::new(proximity::ProximityAllocator {
      max_distance,
      pools: Vec::new(),
    })))
  }

  /// Allocates read-, write- & executable memory close to `origin`.
  pub fn allocate(&self, origin: *const (), size: usize) -> Result<ExecutableMemory> {
    let mut allocator = self.0.lock().unwrap();
    allocator
      .allocate(origin, size)
      .map(|data| ExecutableMemory {
        allocator: self.0.clone(),
        data,
      })
  }
}

/// A handle for allocated proximity memory.
pub struct ExecutableMemory {
  allocator: Arc<Mutex<proximity::ProximityAllocator>>,
  data: proximity::Allocation,
}

impl ExecutableMemory {
  pub fn modify(&mut self, write: impl FnOnce(&mut [u8])) -> Result<()> {
    let _handle = unsafe {
      region::protect_with_handle(
        self.data.deref().as_ptr(),
        self.data.deref().len(),
        region::Protection::READ_WRITE,
      )
    }?;

    write(self.data.deref_mut());
    Ok(())
  }
}

impl Drop for ExecutableMemory {
  fn drop(&mut self) {
    // Release the associated memory map (if unique)
    self.allocator.lock().unwrap().release(&self.data);
  }
}

impl Deref for ExecutableMemory {
  type Target = [u8];

  fn deref(&self) -> &Self::Target {
    self.data.deref()
  }
}
