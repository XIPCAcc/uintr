// UINTR Client - 客户端程序
// 
// 这个程序实现了基于用户态中断（UINTR）的客户端功能

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::os::unix::io::RawFd;
use uintr::{
    UintrError, UintrResult,
    syscall::{uintr_register_handler, uintr_create_fd, uintr_register_sender, senduipi, stui, uintr_wait},
    connection::setup_client_connection,
    UINTR_HANDLER_FLAG_WAITING_ANY, UINTR_WAIT_MAX_USEC,
};

// 声明C语言中断处理程序和全局变量
unsafe extern "C" {
    pub fn ui_handler(ui_frame: *mut uintr::syscall::UintrFrame, vector: u64);
    static mut uintr_received: libc::c_ulong;
}

// 全局状态
static mut CLIENT_UINTRFD: RawFd = -1;
static mut CLIENT_UIPI_INDEX: libc::c_int = -1;

fn get_client_uintrfd() -> RawFd {
    unsafe { CLIENT_UINTRFD }
}

fn set_client_uintrfd(fd: RawFd) {
    unsafe {
        CLIENT_UINTRFD = fd;
    }
}

fn get_client_uipi_index() -> libc::c_int {
    unsafe { CLIENT_UIPI_INDEX }
}

fn set_client_uipi_index(index: libc::c_int) {
    unsafe {
        CLIENT_UIPI_INDEX = index;
    }
}

// 客户端设置
async fn setup_client() -> UintrResult<()> {
    // 注册中断处理程序
    let res = uintr_register_handler(ui_handler, UINTR_HANDLER_FLAG_WAITING_ANY)?;
    println!("Client: Interrupt handler registered successfully: {}", res);

    // 创建客户端uintrfd文件描述符 - 使用向量0
    let client_descriptor = uintr_create_fd(0, 0)?;
    set_client_uintrfd(client_descriptor);
    println!(
        "Client: Created uintrfd with descriptor {} (vector 0)",
        client_descriptor
    );

    // 启用中断
    unsafe {
        stui();
    }
    println!("Client: Interrupts enabled");

    Ok(())
}

// 发送UINTR
fn uintrfd_notify(uipi_index: libc::c_int) -> UintrResult<()> {
    if uipi_index < 0 {
        return Err(UintrError::NotInitialized);
    }

    println!("Sending UINTR with UIPI index: {}", uipi_index);
    unsafe {
        senduipi(uipi_index as u64);
    }
    Ok(())
}

// 客户端通信函数
async fn client_communicate(args: Arguments, test_done: Arc<AtomicBool>) -> UintrResult<()> {
    setup_client().await?;

    println!("Client: Ready for communication");

    // 连接到server的Unix Domain Socket
    let socket_path = "/tmp/uintr.sock";
    let server_fd = setup_client_connection(socket_path, get_client_uintrfd())?;
    println!("Client: Received server file descriptor {}", server_fd);

    // 注册发送者
    let uipi_index = uintr_register_sender(server_fd, 0)?;
    set_client_uipi_index(uipi_index);
    println!("Client: Registered sender for server with UIPI index {}", uipi_index);

    println!("Client: Starting communication for {} messages", args.count);

    let mut message_count = 0;
    let uipi_index = get_client_uipi_index();
    while message_count < args.count && !test_done.load(std::sync::atomic::Ordering::Acquire) {
        println!("Client: Waiting for message #{}", message_count + 1);
        // 等待来自服务端的中断
        while unsafe { uintr_received == 0 } {
            uintr_wait(UINTR_WAIT_MAX_USEC, 0)?;
        }
        unsafe { uintr_received = 0; }
        println!("Client: Received message #{}", message_count + 1);
        // 发送响应中断
        uintrfd_notify(uipi_index)?;
        println!("Client: Sent response for message #{}", message_count + 1);
        message_count += 1;
    }

    println!("Client: Communication complete");

    Ok(())
}

// Arguments structure
#[derive(Clone)]
struct Arguments {
    count: usize,
}

impl Arguments {
    fn parse() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let mut count = 1000;

        for arg in &args[1..] {
            if let Ok(c) = arg.parse() {
                count = c;
            }
        }

        Arguments { count }
    }
}

#[tokio::main]
async fn main() -> UintrResult<()> {
    let args = Arguments::parse();

    println!("Running as client");

    let test_done = Arc::new(AtomicBool::new(false));
    client_communicate(args, test_done).await?;

    Ok(())
}
