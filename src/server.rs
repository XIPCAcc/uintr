// UINTR Server - 服务器端程序
// 
// 这个程序实现了基于用户态中断（UINTR）的服务器端功能

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::os::unix::io::RawFd;
use std::time::Duration;
use uintr::{
    UintrError, UintrResult,
    syscall::{uintr_register_handler, uintr_create_fd, uintr_register_sender, senduipi, stui, uintr_wait},
    connection::setup_server_connection,
    benchmark::Benchmarks,
    UINTR_HANDLER_FLAG_WAITING_ANY, UINTR_WAIT_MAX_USEC,
};

// 声明C语言中断处理程序和全局变量
unsafe extern "C" {
    pub fn ui_handler(ui_frame: *mut uintr::syscall::UintrFrame, vector: u64);
    static mut uintr_received: libc::c_ulong;
}

// 全局状态
static mut SERVER_UINTRFD: RawFd = -1;
static mut CLIENT_UINTRFD: RawFd = -1;
static mut SERVER_UIPI_INDEX: libc::c_int = -1;

fn get_server_uintrfd() -> RawFd {
    unsafe { SERVER_UINTRFD }
}

fn set_server_uintrfd(fd: RawFd) {
    unsafe {
        SERVER_UINTRFD = fd;
    }
}

fn get_server_uipi_index() -> libc::c_int {
    unsafe { SERVER_UIPI_INDEX }
}

fn set_server_uipi_index(index: libc::c_int) {
    unsafe {
        SERVER_UIPI_INDEX = index;
    }
}

// 服务器设置
async fn setup_server() -> UintrResult<()> {
    // 注册中断处理程序
    let res = uintr_register_handler(ui_handler, UINTR_HANDLER_FLAG_WAITING_ANY)?;
    println!("Server: Interrupt handler registered successfully: {}", res);

    // 创建服务器uintrfd文件描述符 - 使用向量0（SERVER_TOKEN）
    let server_descriptor = uintr_create_fd(0, 0)?;
    set_server_uintrfd(server_descriptor);
    println!(
        "Server: Created uintrfd with descriptor {} (vector 0)",
        server_descriptor
    );

    // 启用中断
    unsafe {
        stui();
    }
    println!("Server: Interrupts enabled");

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

// 服务端通信函数
async fn server_communicate(args: Arguments) -> UintrResult<()> {
    setup_server().await?;

    println!("Server: Ready for communication");

    // 创建并监听Unix Domain Socket
    let socket_path = "/tmp/uintr.sock";
    let _ = std::fs::remove_file(socket_path);
    
    let client_fd = setup_server_connection(socket_path, get_server_uintrfd()).await?;
    println!("Server: Received client file descriptor {}", client_fd);

    // 注册发送者
    let uipi_index = uintr_register_sender(client_fd, 0)?;
    set_server_uipi_index(uipi_index);
    println!("Server: Registered sender for client with UIPI index {}", uipi_index);

    println!("Server: Waiting for 2 seconds before starting communication");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // 设置基准测试
    let mut bench = Benchmarks::new();

    println!("Server: Starting communication for {} messages", args.count);

    // 重置总开始时间，确保从实际开始通信时计时
    bench.reset_total_start();
    let uipi_index = get_server_uipi_index();
    for _i in 0..args.count {
        // 开始测量单个操作
        bench.start_operation();

        // 发送中断到客户端
        uintrfd_notify(uipi_index)?;
        
        // 等待响应
        // async_uintr_wait().await?;
        while unsafe { uintr_received == 0 } {
            uintr_wait(UINTR_WAIT_MAX_USEC, 0)?;
        }
        unsafe { uintr_received = 0; }
        // 结束测量单个操作并更新统计
        bench.end_operation();
    }

    // 评估基准测试结果
    let result = bench.evaluate();
    bench.print_results(&result);

    println!("Server: Communication complete");

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

    println!("Running as server");

    // 创建定期唤醒任务，确保异步运行时能够及时响应中断
    tokio::spawn(async {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_micros(5));
        let mut count = 0;
        loop {
            interval.tick().await;
            count += 1;
            if count % 10000000 == 0 {
                println!("Periodic wakeup: count={}", count);
            }
            // 调用process_uintr_wakers来处理中断
            // process_uintr_wakers();
        }
    });

    server_communicate(args).await?;

    // 清理临时文件
    let _ = std::fs::remove_file("/tmp/uintr.sock");

    Ok(())
}
