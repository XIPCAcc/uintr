// 连接模块
// 
// 这个模块提供了服务器和客户端之间的连接建立和文件描述符传递功能

use std::os::unix::io::{RawFd, AsRawFd};
use std::os::unix::net::{UnixListener, UnixStream};
use std::time::Duration;
use crate::{UintrError, UintrResult};

/// 通过Unix Domain Socket发送文件描述符
pub fn send_fd(socket: &UnixStream, fd: RawFd) -> UintrResult<()> {
    unsafe {
        use libc::{msghdr, iovec, sendmsg, CMSG_FIRSTHDR, CMSG_DATA, SOL_SOCKET, SCM_RIGHTS};
        
        let mut buf = [0u8; 1];
        let mut iov = iovec {
            iov_base: buf.as_mut_ptr() as *mut libc::c_void,
            iov_len: 1,
        };
        
        let fd_size = std::mem::size_of::<RawFd>();
        let cmsg_space_size = libc::CMSG_SPACE(fd_size as u32) as usize;
        let mut cmsg_space: Vec<u8> = vec![0; cmsg_space_size];
        
        let msg = msghdr {
            msg_name: std::ptr::null_mut(),
            msg_namelen: 0,
            msg_iov: &mut iov,
            msg_iovlen: 1,
            msg_control: cmsg_space.as_mut_ptr() as *mut libc::c_void,
            msg_controllen: cmsg_space.len(),
            msg_flags: 0,
        };
        
        let cmsg = CMSG_FIRSTHDR(&msg);
        (*cmsg).cmsg_level = SOL_SOCKET;
        (*cmsg).cmsg_type = SCM_RIGHTS;
        (*cmsg).cmsg_len = libc::CMSG_LEN(fd_size as u32) as usize;
        
        let data = CMSG_DATA(cmsg);
        *(data as *mut RawFd) = fd;
        
        let result = sendmsg(socket.as_raw_fd(), &msg, 0);
        if result < 0 {
            return Err(UintrError::SendFdError(format!(
                "send_fd failed: {}",
                std::io::Error::last_os_error()
            )));
        }
    }
    Ok(())
}

/// 通过Unix Domain Socket接收文件描述符
pub fn recv_fd(socket: &UnixStream) -> UintrResult<RawFd> {
    unsafe {
        use libc::{msghdr, iovec, recvmsg, CMSG_FIRSTHDR, CMSG_DATA, SOL_SOCKET, SCM_RIGHTS};
        
        let mut buf = [0u8; 1];
        let mut iov = iovec {
            iov_base: buf.as_mut_ptr() as *mut libc::c_void,
            iov_len: 1,
        };
        
        let fd_size = std::mem::size_of::<RawFd>();
        let cmsg_space_size = libc::CMSG_SPACE(fd_size as u32) as usize;
        let mut cmsg_space: Vec<u8> = vec![0; cmsg_space_size];
        
        let mut msg = msghdr {
            msg_name: std::ptr::null_mut(),
            msg_namelen: 0,
            msg_iov: &mut iov,
            msg_iovlen: 1,
            msg_control: cmsg_space.as_mut_ptr() as *mut libc::c_void,
            msg_controllen: cmsg_space.len(),
            msg_flags: 0,
        };
        
        let result = recvmsg(socket.as_raw_fd(), &mut msg, 0);
        if result < 0 {
            return Err(UintrError::RecvFdError(format!(
                "recv_fd failed: {}",
                std::io::Error::last_os_error()
            )));
        }
        
        let cmsg = CMSG_FIRSTHDR(&msg);
        if cmsg.is_null() || (*cmsg).cmsg_level != SOL_SOCKET || (*cmsg).cmsg_type != SCM_RIGHTS {
            return Err(UintrError::RecvFdError(
                "recv_fd: no file descriptor received".to_string()
            ));
        }
        
        let data = CMSG_DATA(cmsg);
        let fd = *(data as *const RawFd);
        Ok(fd)
    }
}

/// 服务器连接设置
pub struct ServerConnection {
    pub client_socket: Option<UnixStream>,
    pub client_fd: Option<RawFd>,
    pub server_fd: RawFd,
}

impl ServerConnection {
    pub fn new(server_fd: RawFd) -> Self {
        Self {
            client_socket: None,
            client_fd: None,
            server_fd,
        }
    }
    
    /// 等待客户端连接并交换文件描述符
    pub async fn wait_for_client(&mut self, socket_path: &str) -> UintrResult<RawFd> {
        println!("wait_for_client: 开始等待客户端连接，socket 路径: {}", socket_path);
        
        let listener = UnixListener::bind(socket_path).map_err(|e| {
            println!("wait_for_client: 绑定 socket 失败: {}", e);
            UintrError::SocketError(format!("Failed to bind socket: {}", e))
        })?;
        
        println!("wait_for_client: 成功绑定 socket");
        
        listener.set_nonblocking(true).map_err(|e| {
            UintrError::SocketError(format!("Failed to set nonblocking: {}", e))
        })?;
        
        // 等待客户端连接
        for i in 0..1000 {
            match listener.accept() {
                Ok((s, _)) => {
                    println!("wait_for_client: 客户端已连接，尝试次数: {}", i + 1);
                    self.client_socket = Some(s);
                    break;
                }
                Err(e) => {
                    if i % 100 == 0 {
                        println!("wait_for_client: 等待客户端连接... (尝试次数: {}), 错误: {}", i + 1, e);
                    }
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            }
        }
        
        let socket = self.client_socket.as_ref().ok_or_else(|| {
            println!("wait_for_client: 等待客户端连接超时");
            UintrError::Timeout
        })?;
        
        println!("wait_for_client: 接收客户端文件描述符...");
        // 接收client的文件描述符
        let client_fd = recv_fd(socket)?;
        println!("wait_for_client: 已接收客户端文件描述符: {}", client_fd);
        self.client_fd = Some(client_fd);
        
        println!("wait_for_client: 发送服务器文件描述符: {}", self.server_fd);
        // 发送server的文件描述符给client
        send_fd(socket, self.server_fd)?;
        println!("wait_for_client: 已发送服务器文件描述符");
        
        Ok(client_fd)
    }
}

/// 客户端连接设置
pub struct ClientConnection {
    pub server_socket: Option<UnixStream>,
    pub client_fd: RawFd,
    pub server_fd: Option<RawFd>,
}

impl ClientConnection {
    pub fn new(client_fd: RawFd) -> Self {
        Self {
            server_socket: None,
            client_fd,
            server_fd: None,
        }
    }
    
    /// 连接到服务器并交换文件描述符
    pub async fn connect_to_server(&mut self, socket_path: &str) -> UintrResult<RawFd> {
        println!("connect_to_server: 开始连接到服务器，socket 路径: {}", socket_path);
        
        // 连接到server的Unix Domain Socket
        for i in 0..1000 {
            match UnixStream::connect(socket_path) {
                Ok(s) => {
                    println!("connect_to_server: 成功连接到服务器，尝试次数: {}", i + 1);
                    self.server_socket = Some(s);
                    break;
                }
                Err(e) => {
                    if i % 100 == 0 {
                        println!("connect_to_server: 连接失败，重试中... (尝试次数: {}), 错误: {}", i + 1, e);
                    }
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            }
        }
        
        let socket = self.server_socket.as_ref().ok_or_else(|| {
            println!("connect_to_server: 连接超时");
            UintrError::Timeout
        })?;
        
        println!("connect_to_server: 发送客户端文件描述符: {}", self.client_fd);
        // 发送client的文件描述符给server
        send_fd(socket, self.client_fd)?;
        println!("connect_to_server: 已发送客户端文件描述符");
        
        println!("connect_to_server: 接收服务器文件描述符...");
        // 接收server的文件描述符
        let server_fd = recv_fd(socket)?;
        println!("connect_to_server: 已接收服务器文件描述符: {}", server_fd);
        self.server_fd = Some(server_fd);
        
        Ok(server_fd)
    }
}

/// 设置服务器连接（简化版）
pub async fn setup_server_connection(
    socket_path: &str,
    server_fd: RawFd,
) -> UintrResult<RawFd> {
    let mut conn = ServerConnection::new(server_fd);
    conn.wait_for_client(socket_path).await
}

/// 设置客户端连接（简化版）
pub async fn setup_client_connection(
    socket_path: &str,
    client_fd: RawFd,
) -> UintrResult<RawFd> {
    let mut conn = ClientConnection::new(client_fd);
    conn.connect_to_server(socket_path).await
}
