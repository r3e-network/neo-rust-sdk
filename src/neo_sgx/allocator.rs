// SGX Memory Allocator for no_std environments

#![cfg_attr(feature = "sgx", no_std)]

#[cfg(feature = "sgx")]
extern crate sgx_tstd as std;

#[cfg(feature = "sgx")]
use sgx_tstd::alloc::{GlobalAlloc, Layout};
#[cfg(feature = "sgx")]
use sgx_types::*;

use super::SgxError;

/// SGX-compatible memory allocator
#[cfg(feature = "sgx")]
pub struct SgxAllocator {
	heap_base: usize,
	heap_size: usize,
	allocated: core::sync::atomic::AtomicUsize,
}

#[cfg(feature = "sgx")]
impl SgxAllocator {
	/// Create a new SGX allocator with specified heap size
	pub const fn new(heap_base: usize, heap_size: usize) -> Self {
		Self { heap_base, heap_size, allocated: core::sync::atomic::AtomicUsize::new(0) }
	}

	/// Get current memory usage
	pub fn memory_usage(&self) -> usize {
		self.allocated.load(core::sync::atomic::Ordering::Relaxed)
	}

	/// Get available memory
	pub fn available_memory(&self) -> usize {
		self.heap_size - self.memory_usage()
	}
}

#[cfg(feature = "sgx")]
unsafe impl GlobalAlloc for SgxAllocator {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		let size = layout.size();
		let align = layout.align();

		// Simple bump allocator for SGX
		let current = self.allocated.fetch_add(size, core::sync::atomic::Ordering::SeqCst);

		if current + size > self.heap_size {
			// Out of memory
			return core::ptr::null_mut();
		}

		let ptr = (self.heap_base + current) as *mut u8;

		// Ensure alignment
		let aligned_ptr = ((ptr as usize + align - 1) & !(align - 1)) as *mut u8;
		aligned_ptr
	}

	unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
		// Simple allocator doesn't support deallocation
		// In production, implement a more sophisticated allocator
	}
}

/// Static allocator instance for SGX
#[cfg(feature = "sgx")]
#[global_allocator]
static SGX_ALLOCATOR: SgxAllocator = SgxAllocator::new(
	0x1000_0000, // Heap base address
	0x1000_0000, // 256MB heap size
);

/// Initialize the SGX allocator
#[cfg(feature = "sgx")]
pub fn init_allocator() -> Result<(), SgxError> {
	// Perform any necessary initialization
	Ok(())
}

/// Panic handler for no_std environment
#[cfg(all(feature = "sgx", not(test)))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
	use sgx_trts::trts::rsgx_abort;

	// Log panic information if possible
	if let Some(location) = info.location() {
		// In production, log to enclave's secure log
		let _ = location.file();
		let _ = location.line();
	}

	// Abort the enclave
	unsafe { rsgx_abort() }
}

/// OOM handler for no_std environment
#[cfg(feature = "sgx")]
#[alloc_error_handler]
fn oom(_layout: Layout) -> ! {
	use sgx_trts::trts::rsgx_abort;
	unsafe { rsgx_abort() }
}

// Fallback implementations for non-SGX builds
#[cfg(not(feature = "sgx"))]
pub struct SgxAllocator;

#[cfg(not(feature = "sgx"))]
impl SgxAllocator {
	pub fn new(_heap_base: usize, _heap_size: usize) -> Self {
		Self
	}

	pub fn memory_usage(&self) -> usize {
		0
	}

	pub fn available_memory(&self) -> usize {
		usize::MAX
	}
}

#[cfg(not(feature = "sgx"))]
pub fn init_allocator() -> Result<(), SgxError> {
	Ok(())
}
