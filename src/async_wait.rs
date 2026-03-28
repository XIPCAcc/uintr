// 异步等待模块
// 
// 这个模块提供了异步等待UINTR中断的功能

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use crate::UintrResult;
use crate::interrupt::{uintr, get_token};

/// 等待UINTR
pub async fn uintr_wait() -> UintrResult<()> {
    let token = get_token()?;
    uintr(token).await
}

/// UINTR等待器（用于benchmark）
pub struct UintrWait {
    test_done: Arc<AtomicBool>,
}

impl UintrWait {
    pub fn new(test_done: Arc<AtomicBool>) -> Self {
        Self { test_done }
    }
    
    pub async fn wait(&self) -> UintrResult<()> {
        uintr_wait().await
    }
}
