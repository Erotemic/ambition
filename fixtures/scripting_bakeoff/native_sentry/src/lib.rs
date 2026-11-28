//! A trusted native C entry table for one module, the sentry: the native arm
//! of M1. The wire bytes are the loaded-module ABI's (`ambition-ext-3`); only
//! the transport differs from the WebAssembly exports.
//!
//! - Versioned: the host checks [`amb_native_abi`] before it calls anything.
//! - Fixed-width: lengths are `u64` on every target.
//! - Caller-owned buffers: the host passes its output buffer; nothing is
//!   allocated on one side of the boundary and freed on the other.
//! - No unwinding across the boundary: a panic is reported as `u64::MAX`.
//!
//! Trusted means what it says: native code is not sandboxed, has no fuel and
//! can keep a static across calls. The host must load only code it built.

/// The version of this entry table.
pub const AMB_NATIVE_ABI: u32 = 1;

/// The sentry's descriptor, built once: immutable, as code is, so it is not
/// state a call could carry to the next.
fn sentry() -> &'static ambition_extension_sdk::ModuleDescriptor {
    static SENTRY: std::sync::OnceLock<ambition_extension_sdk::ModuleDescriptor> = std::sync::OnceLock::new();
    SENTRY.get_or_init(ambition_content_modules::sentry::module)
}

/// The entry table's version.
#[no_mangle]
pub extern "C" fn amb_native_abi() -> u32 {
    AMB_NATIVE_ABI
}

/// The module list, as `abi::describe` writes it, into the caller's buffer.
/// Returns its length; when that exceeds `out_cap`, nothing is written and
/// the caller asks again with a buffer that large.
///
/// # Safety
///
/// `out` must be valid for writes of `out_cap` bytes.
#[no_mangle]
pub unsafe extern "C" fn amb_native_describe(out: *mut u8, out_cap: u64) -> u64 {
    let bytes = ambition_extension_sdk::abi::describe(std::slice::from_ref(sentry()));
    write_out(&bytes, out, out_cap)
}

/// Run entry `entry` of module `module` on the input bytes, and write the
/// output bytes into the caller's buffer. Returns the output length; when
/// that exceeds `out_cap`, nothing is written and the caller asks again with
/// a buffer that large. `u64::MAX` is a panic or an unknown module.
///
/// # Safety
///
/// `input` must be valid for reads of `input_len` bytes and `out` for writes
/// of `out_cap` bytes.
#[no_mangle]
pub unsafe extern "C" fn amb_native_invoke(
    module: u32,
    entry: u32,
    input: *const u8,
    input_len: u64,
    out: *mut u8,
    out_cap: u64,
) -> u64 {
    if module != 0 {
        return u64::MAX;
    }
    let input = std::slice::from_raw_parts(input, input_len as usize);
    let ran = std::panic::catch_unwind(|| {
        ambition_extension_sdk::abi::invoke_one(sentry(), entry, input)
    });
    match ran {
        Ok(bytes) => write_out(&bytes, out, out_cap),
        Err(_) => u64::MAX,
    }
}

unsafe fn write_out(bytes: &[u8], out: *mut u8, out_cap: u64) -> u64 {
    if bytes.len() as u64 <= out_cap {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len());
    }
    bytes.len() as u64
}
