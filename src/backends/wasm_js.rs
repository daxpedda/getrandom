//! Implementation for WASM based on Web and Node.js

/// Proc-macro is not hygienic.
/// See <https://github.com/rustwasm/wasm-bindgen/pull/4315>.
#[cfg(feature = "std")]
extern crate std;

use crate::Error;
use core::mem::MaybeUninit;
#[cfg(feature = "std")]
use std::thread_local;

pub use crate::util::{inner_u32, inner_u64};

#[cfg(not(all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none"))))]
compile_error!("`wasm_js` backend can be enabled only for OS-less WASM targets!");

use js_sys::Uint8Array;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

// Size of our temporary Uint8Array buffer used with WebCrypto methods
// Maximum is 65536 bytes see https://developer.mozilla.org/en-US/docs/Web/API/Crypto/getRandomValues
const CRYPTO_BUFFER_SIZE: u16 = 256;

pub fn fill_inner(dest: &mut [MaybeUninit<u8>]) -> Result<(), Error> {
    CRYPTO.with(|crypto| {
        let crypto = crypto.as_ref().ok_or(Error::WEB_CRYPTO)?;

        // getRandomValues does not work with all types of WASM memory,
        // so we initially write to browser memory to avoid exceptions.
        let buf = Uint8Array::new_with_length(CRYPTO_BUFFER_SIZE.into());
        for chunk in dest.chunks_mut(CRYPTO_BUFFER_SIZE.into()) {
            let chunk_len: u32 = chunk
                .len()
                .try_into()
                .expect("chunk length is bounded by CRYPTO_BUFFER_SIZE");
            // The chunk can be smaller than buf's length, so we call to
            // JS to create a smaller view of buf without allocation.
            let sub_buf = buf.subarray(0, chunk_len);

            if crypto.get_random_values(&sub_buf).is_err() {
                return Err(Error::WEB_GET_RANDOM_VALUES);
            }

            // SAFETY: `sub_buf`'s length is the same length as `chunk`
            unsafe { sub_buf.raw_copy_to_ptr(chunk.as_mut_ptr().cast::<u8>()) };
        }
        Ok(())
    })
}

#[wasm_bindgen]
extern "C" {
    // Web Crypto API: Crypto interface (https://www.w3.org/TR/WebCryptoAPI/)
    type Crypto;
    // Holds the global `Crypto` object.
    #[wasm_bindgen(thread_local_v2, js_namespace = globalThis, js_name = crypto)]
    static CRYPTO: Option<Crypto>;
    // Crypto.getRandomValues()
    #[wasm_bindgen(method, js_name = getRandomValues, catch)]
    fn get_random_values(this: &Crypto, buf: &Uint8Array) -> Result<(), JsValue>;
}
