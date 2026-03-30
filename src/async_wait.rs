use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use std::task::{Context, Poll, Waker};
use std::pin::Pin;
use std::future::Future;

use crate::UintrResult;
use crate::UintrError;
use crate::handler::set_handler_token;

#[derive(Clone)]
pub struct UintrToken {
    inner: Arc<Inner>,
    name: String,
}

struct Inner {
    pending: Mutex<bool>,
    waker: Mutex<Option<Waker>>,
}

impl UintrToken {
    pub fn new(name: &str) -> Self {
        Self {
            inner: Arc::new(Inner {
                pending: Mutex::new(false),
                waker: Mutex::new(None),
            }),
            name: name.to_string(),
        }
    }
    
    pub fn name(&self) -> &str {
        &self.name
    }
    
    pub fn set_pending(&self) {
        let mut pending = self.inner.pending.lock().unwrap();
        *pending = true;
    }
    
    pub fn clear_pending(&self) {
        let mut pending = self.inner.pending.lock().unwrap();
        *pending = false;
    }
}

pub struct UintrFuture {
    token: UintrToken,
}

impl Future for UintrFuture {
    type Output = UintrResult<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut pending = self.token.inner.pending.lock().unwrap();
        if *pending {
            *pending = false;
            Poll::Ready(Ok(()))
        } else {
            *self.token.inner.waker.lock().unwrap() = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

pub async fn uintr(token: UintrToken) -> UintrResult<()> {
    UintrFuture { token }.await
}

static mut TOKEN: Option<UintrToken> = None;

pub fn get_token() -> UintrResult<UintrToken> {
    unsafe {
        TOKEN.clone().ok_or(UintrError::NotInitialized)
    }
}

pub fn init_token(name: &str) -> UintrToken {
    let token = UintrToken::new(name);
    unsafe {
        TOKEN = Some(token.clone());
    }
    set_handler_token(token.clone());
    token
}

pub fn process_uintr_wakers(token: UintrToken) -> u32 {
    let should_wake = {
        let pending = token.inner.pending.lock().unwrap();
        *pending
    };
    
    if should_wake {
        if let Some(waker) = token.inner.waker.lock().unwrap().take() {
            waker.wake();
            return 1;
        }
    }
    0
}

pub async fn uintr_wait() -> UintrResult<()> {
    let token = get_token()?;
    uintr(token).await
}
