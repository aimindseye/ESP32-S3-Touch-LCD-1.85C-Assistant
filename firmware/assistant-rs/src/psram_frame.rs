use anyhow::{bail, Result};
use core::{mem::size_of, ptr::NonNull, slice};
use esp_idf_sys::{
    heap_caps_free, heap_caps_malloc, MALLOC_CAP_8BIT, MALLOC_CAP_SPIRAM,
};

pub struct PsramFrame {
    ptr: NonNull<u16>,
    len_words: usize,
}

impl PsramFrame {
    pub fn new_u16(len_words: usize) -> Result<Self> {
        let bytes = len_words
            .checked_mul(size_of::<u16>())
            .ok_or_else(|| anyhow::anyhow!("framebuffer size overflow"))?;

        let raw = unsafe { heap_caps_malloc(bytes, MALLOC_CAP_SPIRAM | MALLOC_CAP_8BIT) as *mut u16 };

        let ptr = NonNull::new(raw).ok_or_else(|| anyhow::anyhow!("PSRAM framebuffer alloc failed"))?;
        unsafe {
            core::ptr::write_bytes(ptr.as_ptr(), 0, len_words);
        }

        Ok(Self { ptr, len_words })
    }

    pub fn as_mut_slice(&mut self) -> &mut [u16] {
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len_words) }
    }
}

impl Drop for PsramFrame {
    fn drop(&mut self) {
        unsafe { heap_caps_free(self.ptr.as_ptr() as *mut _) };
    }
}