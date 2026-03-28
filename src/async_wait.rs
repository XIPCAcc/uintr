// 异步等待模块
// 
// 这个模块提供了服务器和客户端的异步等待UINTR中断的功能

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use crate::UintrResult;
use crate::interrupt::{uintr, get_server_token, get_client_token};

/// 服务器等待UINTR
pub async fn server_uintr_wait() -> UintrResult<()> {
    let token = get_server_token()?;
    uintr(token).await
}

/// 客户端等待UINTR
pub async fn client_uintr_wait() -> UintrResult<()> {
    let token = get_client_token()?;
    uintr(token).await
}

/// 服务器UINTR等待器（用于benchmark）
pub struct ServerUintrWait {
    test_done: Arc<AtomicBool>,
}

impl ServerUintrWait {
    pub fn new(test_done: Arc<AtomicBool>) -> Self {
        Self { test_done }
    }
    
    pub async fn wait(&self) -> UintrResult<()> {
        server_uintr_wait().await
    }
}

/// 客户端UINTR等待器（用于benchmark）
pub struct ClientUintrWait {
    test_done: Arc<AtomicBool>,
}

impl ClientUintrWait {
    pub fn new(test_done: Arc<AtomicBool>) -> Self {
        Self { test_done }
    }
    
    pub async fn wait(&self) -> UintrResult<()> {
        client_uintr_wait().await
    }
}
