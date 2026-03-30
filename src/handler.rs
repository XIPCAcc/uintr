use libc::c_char;
use crate::async_wait::UintrToken;

#[no_mangle]
pub extern "C" fn rust_interrupt_callback(_handler_name: *const c_char, _vector: u64) {
    unsafe {
        if let Some(ref token) = TOKEN {
            token.set_pending();
        }
    }
}

static mut TOKEN: Option<UintrToken> = None;

pub fn set_handler_token(token: UintrToken) {
    unsafe {
        TOKEN = Some(token);
    }
}

