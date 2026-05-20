use libc::c_char;
use uintr_core::{notify_global_uintr, UintrToken};

#[no_mangle]
pub extern "C" fn rust_interrupt_callback(_handler_name: *const c_char, _vector: u64) {
    // if notify_global_uintr().is_err() {
        unsafe {
            // println!("callback");
            if let Some(ref token) = TOKEN {
                // println!("callback: set_pending");
                token.set_pending();
            }
        }
    // }
}

static mut TOKEN: Option<UintrToken> = None;

pub fn set_handler_token(token: UintrToken) {
    unsafe {
        TOKEN = Some(token);
    }
}
