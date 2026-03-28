// 中断处理模块
// 
// 这个模块提供了中断处理的核心功能，包括UintrToken和异步等待机制

use std::sync::Arc;
use std::sync::Mutex;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use crate::{UintrError, UintrResult};

// ============================================================================
// 全局状态
// ============================================================================

/// UintrToken实例
static mut TOKEN_OBJ: Option<UintrToken> = None;

/// 初始化标志
static mut INITIALIZED: bool = false;

// ============================================================================
// 数据结构
// ============================================================================

/// 表示某个 UINTR 中断源的句柄
/// 
/// 这个结构体封装了中断状态和waker，用于异步等待中断
#[derive(Clone)]
pub struct UintrToken {
    inner: Arc<Inner>,
    name: String,
}

/// 内部状态
struct Inner {
    /// 是否已经收到一次中断
    pending: Mutex<bool>,
    /// 当前在等这个中断的任务的 waker（最多一个）
    waker: Mutex<Option<Waker>>,
}

impl UintrToken {
    /// 创建新的UintrToken
    /// 
    /// # 参数
    /// 
    /// * `name` - Token名称，用于调试
    /// 
    /// # 返回
    /// 
    /// 返回新创建的UintrToken实例
    pub fn new(name: &str) -> Self {
        Self {
            inner: Arc::new(Inner {
                pending: Mutex::new(false),
                waker: Mutex::new(None),
            }),
            name: name.to_string(),
        }
    }
    
    /// 获取Token名称
    pub fn name(&self) -> &str {
        &self.name
    }
}

// ============================================================================
// Future实现
// ============================================================================

/// UINTR 异步 Future
/// 
/// 这个结构体实现了Future trait，用于异步等待中断
pub struct UintrFuture {
    token: UintrToken,
}

impl Future for UintrFuture {
    type Output = UintrResult<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // 先检查是否有 pending 中断，避免时序问题
        let mut pending = self.token.inner.pending.lock().unwrap();
        if *pending {
            *pending = false;
            Poll::Ready(Ok(()))
        } else {
            // 没有 pending，保存 waker 并返回 Pending
            *self.token.inner.waker.lock().unwrap() = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

// ============================================================================
// 公共API
// ============================================================================

/// 异步等待 UINTR 中断
/// 
/// # 参数
/// 
/// * `token` - UintrToken实例
/// 
/// # 返回
/// 
/// 成功返回Ok(())，失败返回错误
pub async fn uintr(token: UintrToken) -> UintrResult<()> {
    UintrFuture { token }.await
}

/// 初始化UintrToken
/// 
/// 这个函数必须在启动时调用，用于初始化中断状态
pub fn init_token() {
    unsafe {
        TOKEN_OBJ = Some(UintrToken::new("UINTR"));
        INITIALIZED = true;
    }
}

/// 获取UintrToken
/// 
/// # 返回
/// 
/// 成功返回UintrToken实例，失败返回NotInitialized错误
pub fn get_token() -> UintrResult<UintrToken> {
    unsafe {
        if INITIALIZED {
            Ok(TOKEN_OBJ.as_ref().expect("TOKEN_OBJ not initialized").clone())
        } else {
            Err(UintrError::NotInitialized)
        }
    }
}

// ============================================================================
// C回调函数
// ============================================================================

/// Rust回调函数，供C代码调用
/// 
/// 这个函数在中断处理程序中被调用，用于设置中断标志
/// 
/// # 参数
/// 
/// * `_handler_name` - 处理程序名称（未使用）
/// * `vector` - 中断向量号（未使用）
#[no_mangle]
pub extern "C" fn rust_interrupt_callback(_handler_name: *const libc::c_char, _vector: u64) {
    unsafe {
        if INITIALIZED {
            if let Some(ref token) = TOKEN_OBJ {
                let mut pending = token.inner.pending.lock().unwrap();
                *pending = true;
            }
        }
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 供 Tokio IO Driver 调用的函数
/// 
/// 检查是否有待处理的中断
/// 
/// # 返回
/// 
/// 如果有待处理的中断返回true，否则返回false
#[no_mangle]
pub extern "C" fn check_uintr_pending() -> bool {
    unsafe {
        TOKEN_OBJ.as_ref().map(|token| {
            *token.inner.pending.lock().unwrap()
        }).unwrap_or(false)
    }
}

/// 处理UINTR wakers
/// 
/// 这个函数检查是否有待处理的中断，如果有则唤醒相应的waker
/// 
/// # 返回
/// 
/// 返回唤醒的waker数量
#[no_mangle]
pub extern "C" fn process_uintr_wakers() -> u32 {
    unsafe {
        let mut waker_count = 0;
        
        if let Some(ref token) = TOKEN_OBJ {
            let should_wake = {
                let pending = token.inner.pending.lock().unwrap();
                *pending
            };
            
            if should_wake {
                if let Some(waker) = token.inner.waker.lock().unwrap().take() {
                    waker.wake();
                    waker_count += 1;
                }
            }
        }
        
        waker_count
    }
}
