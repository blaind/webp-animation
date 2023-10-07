use std::{mem, ops::Deref, slice};

use libwebp_sys as webp;

/// A safe wrapper for WebP bytedata. Consider as `&[u8]` (implements [`Deref`])
#[derive(Debug)]
pub struct WebPData {
    data: webp::WebPData,
}

impl WebPData {
    pub(crate) fn new() -> Self {
        let data = unsafe {
            let mut data = mem::zeroed();
            webp::WebPDataInit(&mut data);
            data
        };

        WebPData { data }
    }

    pub(crate) fn inner_ref(&mut self) -> &mut webp::WebPData {
        &mut self.data
    }

    fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.data.bytes, self.data.size) }
    }
}

/// SAFETY: Sending `WebPData` to another thread is safe until there is no way to share internal
/// pointer without borrowing in safe code
unsafe impl Send for WebPData {}

/// SAFETY: Sharing `WebPData` via borrowing is safe until there is no way to share internal
/// pointer without borrowing in safe code
unsafe impl Sync for WebPData {}

impl Drop for WebPData {
    fn drop(&mut self) {
        unsafe { libwebp_sys::WebPDataClear(self.inner_ref()) }
    }
}

impl Deref for WebPData {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl AsRef<[u8]> for WebPData {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoder() {
        let data = WebPData::new();
        assert_eq!(data.len(), 0);
    }
}
