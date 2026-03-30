// UINTR Library - 用户态中断库
// 
// 这个库提供了基于用户态中断（UINTR）的双向通信功能
// 可以被外部crate直接使用

pub mod syscall;
pub mod handler;
pub mod connection;
pub mod benchmark;
pub mod async_wait;

// 重新导出常用的类型和函数
pub use syscall::{UintrFrame, senduipi, stui, clui, uiret};
pub use connection::{send_fd, recv_fd, setup_server_connection, setup_client_connection};
pub use benchmark::{Benchmarks, BenchmarkResult};

// ============================================================================
// 系统调用号
// ============================================================================

/// 注册UINTR中断处理程序
pub const __NR_UINTR_REGISTER_HANDLER: libc::c_long = 471;

/// 注销UINTR中断处理程序
pub const __NR_UINTR_UNREGISTER_HANDLER: libc::c_long = 472;

/// 创建UINTR文件描述符
pub const __NR_UINTR_CREATE_FD: libc::c_long = 473;

/// 注册UINTR发送者
pub const __NR_UINTR_REGISTER_SENDER: libc::c_long = 474;

/// 注销UINTR发送者
pub const __NR_UINTR_UNREGISTER_SENDER: libc::c_long = 475;

/// UINTR等待系统调用
pub const __NR_UINTR_WAIT: libc::c_long = 476;

// ============================================================================
// UINTR标志和常量
// ============================================================================

/// UINTR处理程序标志：等待任意中断
pub const UINTR_HANDLER_FLAG_WAITING_ANY: libc::c_int = 0x3000;

/// UINTR等待超时时间（微秒）
pub const UINTR_WAIT_MAX_USEC: libc::c_long = 10_000_000; // 10 seconds

/// UINTR中断向量（服务器和客户端都使用0）
pub const UINTR_VECTOR: u64 = 0;

// 错误类型
#[derive(Debug)]
pub enum UintrError {
    SyscallError(String),
    RegisterHandlerError(String),
    CreateFdError(String),
    RegisterSenderError(String),
    SendFdError(String),
    RecvFdError(String),
    SocketError(String),
    Timeout,
    NotInitialized,
}

impl std::fmt::Display for UintrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UintrError::SyscallError(msg) => write!(f, "System call failed: {}", msg),
            UintrError::RegisterHandlerError(msg) => write!(f, "Failed to register handler: {}", msg),
            UintrError::CreateFdError(msg) => write!(f, "Failed to create FD: {}", msg),
            UintrError::RegisterSenderError(msg) => write!(f, "Failed to register sender: {}", msg),
            UintrError::SendFdError(msg) => write!(f, "Failed to send file descriptor: {}", msg),
            UintrError::RecvFdError(msg) => write!(f, "Failed to receive file descriptor: {}", msg),
            UintrError::SocketError(msg) => write!(f, "Socket error: {}", msg),
            UintrError::Timeout => write!(f, "Timeout"),
            UintrError::NotInitialized => write!(f, "Not initialized"),
        }
    }
}

impl std::error::Error for UintrError {}

// 结果类型
pub type UintrResult<T> = Result<T, UintrError>;
