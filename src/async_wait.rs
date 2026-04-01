use std::task::{Context, Poll};
use std::pin::Pin;
use std::future::Future;
use std::sync::atomic::Ordering;

use crate::UintrResult;
use crate::UintrError;
use crate::handler::set_handler_token;

pub use uintr_core::{UintrToken, process_uintr_wakers};

pub struct UintrFuture {
    token: UintrToken,
}

impl Future for UintrFuture {
    type Output = UintrResult<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let seq = self.token.inner.seq.load(Ordering::Acquire);
        let consumed = self.token.inner.consumed_seq.load(Ordering::Acquire);

        if seq != consumed {
            self.token.inner.consumed_seq.store(seq, Ordering::Release);
            return Poll::Ready(Ok(()));
        }

        *self.token.inner.waker.lock().unwrap() = Some(cx.waker().clone());

        let seq = self.token.inner.seq.load(Ordering::Acquire);
        let consumed = self.token.inner.consumed_seq.load(Ordering::Acquire);
        if seq != consumed {
            self.token.inner.consumed_seq.store(seq, Ordering::Release);
            return Poll::Ready(Ok(()));
        }

        Poll::Pending
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
    UintrToken::set_global_token(token.clone());
    token
}

pub async fn uintr_wait() -> UintrResult<()> {
    let token = get_token()?;
    uintr(token).await
}
