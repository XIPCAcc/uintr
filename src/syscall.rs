// 系统调用封装模块
// 
// 这个模块封装了所有与UINTR相关的系统调用

use libc::{c_int, c_long, syscall};
use std::os::unix::io::RawFd;
use crate::{
    UintrError, UintrResult,
    __NR_UINTR_REGISTER_HANDLER,
    __NR_UINTR_UNREGISTER_HANDLER,
    __NR_UINTR_CREATE_FD,
    __NR_UINTR_REGISTER_SENDER,
    __NR_UINTR_UNREGISTER_SENDER,
    __NR_UINTR_WAIT,
};

// UINTR栈帧结构（必须与内核一致）
#[repr(C)]
pub struct UintrFrame {
    pub rip: u64,    // 中断返回地址
    pub rflags: u64, // 标志寄存器
    pub rsp: u64,    // 栈指针
}

/// 注册UINTR中断处理程序
pub fn uintr_register_handler(
    handler: unsafe extern "C" fn(*mut UintrFrame, u64),
    flags: c_int,
) -> UintrResult<c_int> {
    let result = unsafe { syscall(__NR_UINTR_REGISTER_HANDLER, handler, flags) as c_int };
    if result < 0 {
        Err(UintrError::RegisterHandlerError(format!(
            "uintr_register_handler failed: {}",
            std::io::Error::last_os_error()
        )))
    } else {
        Ok(result)
    }
}

/// 创建UINTR文件描述符
pub fn uintr_create_fd(vector: c_int, flags: c_int) -> UintrResult<RawFd> {
    let result = unsafe { syscall(__NR_UINTR_CREATE_FD, vector, flags) as RawFd };
    if result < 0 {
        Err(UintrError::CreateFdError(format!(
            "uintr_create_fd failed: {}",
            std::io::Error::last_os_error()
        )))
    } else {
        Ok(result)
    }
}

/// 注册UINTR发送者
pub fn uintr_register_sender(fd: RawFd, flags: c_int) -> UintrResult<c_int> {
    let result = unsafe { syscall(__NR_UINTR_REGISTER_SENDER, fd, flags) as c_int };
    if result < 0 {
        Err(UintrError::RegisterSenderError(format!(
            "uintr_register_sender failed: {}",
            std::io::Error::last_os_error()
        )))
    } else {
        Ok(result)
    }
}

/// 注销UINTR中断处理程序
pub fn uintr_unregister_handler() -> UintrResult<c_int> {
    let result = unsafe { syscall(__NR_UINTR_UNREGISTER_HANDLER) as c_int };
    if result < 0 {
        Err(UintrError::SyscallError(format!(
            "uintr_unregister_handler failed: {}",
            std::io::Error::last_os_error()
        )))
    } else {
        Ok(result)
    }
}

/// 注销UINTR发送者
pub fn uintr_unregister_sender(fd: RawFd) -> UintrResult<c_int> {
    let result = unsafe { syscall(__NR_UINTR_UNREGISTER_SENDER, fd) as c_int };
    if result < 0 {
        Err(UintrError::SyscallError(format!(
            "uintr_unregister_sender failed: {}",
            std::io::Error::last_os_error()
        )))
    } else {
        Ok(result)
    }
}

/// UINTR等待系统调用
pub fn uintr_wait(usec: c_long, flags: c_int) -> UintrResult<()> {
    let result = unsafe { syscall(__NR_UINTR_WAIT, usec, flags) };
    if result < 0 {
        let err = std::io::Error::last_os_error();
        if err.kind() == std::io::ErrorKind::Interrupted {
            Ok(())
        } else {
            Err(UintrError::SyscallError(format!(
                "uintr_wait failed: {}",
                err
            )))
        }
    } else {
        Ok(())
    }
}

/// 发送用户中断
/// 
/// # Safety
/// - The caller must ensure that the index is valid and registered
/// - Sending to an invalid index may lead to undefined behavior
pub unsafe fn senduipi(index: u64) {
    unsafe {
        core::arch::asm!(
            "senduipi {0}",
            in(reg) index,
            options(nostack, nomem)
        );
    }
}

/// 启用用户中断
/// 
/// # Safety
/// - The caller must ensure that user interrupts are properly initialized
/// - Enabling user interrupts without proper setup may lead to unexpected behavior
pub unsafe fn stui() {
    unsafe {
        core::arch::asm!("stui");
    }
}

/// 禁用用户中断
/// 
/// # Safety
/// - The caller must ensure that disabling user interrupts won't break critical sections
/// - This function should be called in pairs with `stui`
pub unsafe fn clui() {
    unsafe {
        core::arch::asm!("clui");
    }
}

/// 用户中断返回指令
/// 
/// # Safety
/// - This function should only be called from within a user interrupt handler
/// - It must be the last instruction executed in the interrupt handler
pub unsafe fn uiret() {
    unsafe {
        core::arch::asm!("uiret", options(noreturn));
    }
}
