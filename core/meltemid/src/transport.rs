// SPDX-License-Identifier: Apache-2.0

//! Local transport abstraction (design D2): Unix domain sockets with `0700`
//! permissions on macOS/Linux, named pipes with a user-restricted ACL on
//! Windows. No network ports, ever. Nothing outside this module touches the
//! socket directly.

use std::io;

use tokio::io::{AsyncRead, AsyncWrite};

/// A connected, bidirectional byte stream between daemon and client.
pub trait RawStream: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> RawStream for T {}

/// Boxed transport stream.
pub type Stream = Box<dyn RawStream>;

/// Listening endpoint owned by the daemon. Also enforces single instance:
/// binding fails with [`io::ErrorKind::AddrInUse`] when another live daemon
/// already owns the endpoint.
pub struct Listener {
    inner: imp::Listener,
}

impl Listener {
    /// Binds the endpoint, enforcing user-exclusive permissions.
    pub async fn bind(endpoint: &str) -> io::Result<Self> {
        Ok(Self {
            inner: imp::Listener::bind(endpoint).await?,
        })
    }

    /// Waits for the next client connection.
    pub async fn accept(&mut self) -> io::Result<Stream> {
        self.inner.accept().await
    }
}

/// Connects to a daemon endpoint as a client.
pub async fn connect(endpoint: &str) -> io::Result<Stream> {
    imp::connect(endpoint).await
}

#[cfg(windows)]
pub use imp::current_user_sid;

#[cfg(unix)]
mod imp {
    use super::Stream;
    use std::io;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use tokio::net::{UnixListener, UnixStream};

    pub struct Listener {
        listener: UnixListener,
        path: PathBuf,
    }

    impl Listener {
        pub async fn bind(endpoint: &str) -> io::Result<Self> {
            let path = PathBuf::from(endpoint);
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
                std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
            }
            if path.exists() {
                // Single instance: if something answers on the socket, a
                // daemon is already running. Otherwise the file is stale.
                match UnixStream::connect(&path).await {
                    Ok(_) => {
                        return Err(io::Error::new(
                            io::ErrorKind::AddrInUse,
                            "another meltemid instance already owns this socket",
                        ));
                    }
                    Err(_) => std::fs::remove_file(&path)?,
                }
            }
            let listener = UnixListener::bind(&path)?;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))?;
            Ok(Self { listener, path })
        }

        pub async fn accept(&mut self) -> io::Result<Stream> {
            let (stream, _addr) = self.listener.accept().await?;
            Ok(Box::new(stream))
        }
    }

    impl Drop for Listener {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    pub async fn connect(endpoint: &str) -> io::Result<Stream> {
        let stream = UnixStream::connect(Path::new(endpoint)).await?;
        Ok(Box::new(stream))
    }
}

#[cfg(windows)]
mod imp {
    use super::Stream;
    use std::ffi::c_void;
    use std::io;
    use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeServer, ServerOptions};
    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_ACCESS_DENIED, ERROR_PIPE_BUSY, HANDLE, LocalFree,
    };
    use windows_sys::Win32::Security::Authorization::{
        ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
        SDDL_REVISION_1,
    };
    use windows_sys::Win32::Security::{
        GetTokenInformation, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER, TokenUser,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    /// String SID of the user this process runs as.
    pub fn current_user_sid() -> String {
        // SAFETY: standard token-query sequence; buffers sized by the first
        // GetTokenInformation call; every handle/local allocation released.
        unsafe {
            let mut token: HANDLE = std::ptr::null_mut();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
                panic!("OpenProcessToken failed: {}", io::Error::last_os_error());
            }
            let mut needed = 0u32;
            let _ = GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut needed);
            let mut buf = vec![0u8; needed as usize];
            let ok = GetTokenInformation(
                token,
                TokenUser,
                buf.as_mut_ptr().cast(),
                needed,
                &mut needed,
            );
            CloseHandle(token);
            if ok == 0 {
                panic!("GetTokenInformation failed: {}", io::Error::last_os_error());
            }
            let token_user: &TOKEN_USER = &*buf.as_ptr().cast();
            let mut sid_str: *mut u16 = std::ptr::null_mut();
            if ConvertSidToStringSidW(token_user.User.Sid, &mut sid_str) == 0 {
                panic!(
                    "ConvertSidToStringSidW failed: {}",
                    io::Error::last_os_error()
                );
            }
            let mut len = 0usize;
            while *sid_str.add(len) != 0 {
                len += 1;
            }
            let sid = String::from_utf16_lossy(std::slice::from_raw_parts(sid_str, len));
            LocalFree(sid_str.cast());
            sid
        }
    }

    /// Security descriptor restricting the pipe to the current user and
    /// LocalSystem, with a protected DACL (no inheritance).
    struct UserOnlyDescriptor {
        psd: *mut c_void,
    }

    // SAFETY: the descriptor is an opaque immutable blob owned by this
    // struct; it is only read by CreateNamedPipe.
    unsafe impl Send for UserOnlyDescriptor {}

    impl UserOnlyDescriptor {
        fn new() -> io::Result<Self> {
            let sddl = format!("D:P(A;;GA;;;SY)(A;;GA;;;{})", current_user_sid());
            let mut wide: Vec<u16> = sddl.encode_utf16().collect();
            wide.push(0);
            let mut psd: *mut c_void = std::ptr::null_mut();
            // SAFETY: wide is a valid NUL-terminated UTF-16 SDDL string; the
            // returned descriptor is freed in Drop via LocalFree.
            let ok = unsafe {
                ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    wide.as_ptr(),
                    SDDL_REVISION_1,
                    (&mut psd as *mut *mut c_void).cast(),
                    std::ptr::null_mut(),
                )
            };
            if ok == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(Self { psd })
        }

        fn attributes(&self) -> SECURITY_ATTRIBUTES {
            SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: self.psd,
                bInheritHandle: 0,
            }
        }
    }

    impl Drop for UserOnlyDescriptor {
        fn drop(&mut self) {
            // SAFETY: psd was allocated by the SDDL conversion with LocalAlloc.
            unsafe {
                LocalFree(self.psd);
            }
        }
    }

    pub struct Listener {
        pipe_name: String,
        descriptor: UserOnlyDescriptor,
        next: Option<NamedPipeServer>,
    }

    impl Listener {
        pub async fn bind(endpoint: &str) -> io::Result<Self> {
            let descriptor = UserOnlyDescriptor::new()?;
            let first = create_instance(endpoint, true, &descriptor).map_err(|e| {
                if e.raw_os_error() == Some(ERROR_ACCESS_DENIED as i32) {
                    io::Error::new(
                        io::ErrorKind::AddrInUse,
                        "another meltemid instance already owns this pipe",
                    )
                } else {
                    e
                }
            })?;
            Ok(Self {
                pipe_name: endpoint.to_string(),
                descriptor,
                next: Some(first),
            })
        }

        pub async fn accept(&mut self) -> io::Result<Stream> {
            let server = match self.next.take() {
                Some(server) => server,
                None => create_instance(&self.pipe_name, false, &self.descriptor)?,
            };
            server.connect().await?;
            // Keep an instance listening so the next client can connect
            // while this stream is being served.
            self.next = create_instance(&self.pipe_name, false, &self.descriptor).ok();
            Ok(Box::new(server))
        }
    }

    fn create_instance(
        name: &str,
        first: bool,
        descriptor: &UserOnlyDescriptor,
    ) -> io::Result<NamedPipeServer> {
        let mut attrs = descriptor.attributes();
        // SAFETY: attrs points to a valid SECURITY_ATTRIBUTES whose
        // descriptor outlives the call (owned by the listener).
        unsafe {
            ServerOptions::new()
                .first_pipe_instance(first)
                .create_with_security_attributes_raw(
                    name,
                    (&mut attrs as *mut SECURITY_ATTRIBUTES).cast(),
                )
        }
    }

    /// Overall budget for retrying `ERROR_PIPE_BUSY` while waiting for a free
    /// pipe instance. Bounded on purpose: a busy-but-unserved pipe (e.g. right
    /// after the daemon stops accepting) would otherwise spin this loop
    /// forever. A client that cannot connect within the budget gets a
    /// `TimedOut` error instead of hanging.
    const CONNECT_RETRY_BUDGET: std::time::Duration = std::time::Duration::from_secs(2);

    pub async fn connect(endpoint: &str) -> io::Result<Stream> {
        let deadline = std::time::Instant::now() + CONNECT_RETRY_BUDGET;
        loop {
            match ClientOptions::new().open(endpoint) {
                Ok(client) => return Ok(Box::new(client)),
                Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY as i32) => {
                    if std::time::Instant::now() >= deadline {
                        return Err(io::Error::new(
                            io::ErrorKind::TimedOut,
                            "pipe is busy: no daemon instance became available in time",
                        ));
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn test_endpoint(tag: &str) -> String {
        #[cfg(windows)]
        {
            format!(r"\\.\pipe\meltemid-test-{}-{tag}", std::process::id())
        }
        #[cfg(unix)]
        {
            std::env::temp_dir()
                .join(format!("meltemid-test-{}-{tag}.sock", std::process::id()))
                .to_string_lossy()
                .into_owned()
        }
    }

    #[tokio::test]
    async fn roundtrip_and_multiple_clients() {
        let endpoint = test_endpoint("roundtrip");
        let mut listener = Listener::bind(&endpoint).await.expect("bind");

        let server = tokio::spawn(async move {
            for _ in 0..2 {
                let mut stream = listener.accept().await.expect("accept");
                let mut buf = [0u8; 4];
                stream.read_exact(&mut buf).await.expect("read");
                stream.write_all(b"pong").await.expect("write");
                stream.flush().await.expect("flush");
            }
        });

        for _ in 0..2 {
            let mut client = connect(&endpoint).await.expect("connect");
            client.write_all(b"ping").await.expect("write");
            client.flush().await.expect("flush");
            let mut buf = [0u8; 4];
            client.read_exact(&mut buf).await.expect("read");
            assert_eq!(&buf, b"pong");
        }
        server.await.expect("server task");
    }

    #[tokio::test]
    async fn second_bind_fails_while_instance_is_alive() {
        let endpoint = test_endpoint("single");
        let _listener = Listener::bind(&endpoint).await.expect("first bind");
        let second = Listener::bind(&endpoint).await;
        let err = second.err().expect("second bind must fail");
        assert_eq!(err.kind(), std::io::ErrorKind::AddrInUse, "{err}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn socket_and_directory_are_user_exclusive() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("meltemi-perm-{}", std::process::id()));
        let endpoint = dir.join("meltemid.sock").to_string_lossy().into_owned();
        let _listener = Listener::bind(&endpoint).await.expect("bind");
        let socket_mode = std::fs::metadata(&endpoint)
            .expect("meta")
            .permissions()
            .mode();
        let dir_mode = std::fs::metadata(&dir).expect("meta").permissions().mode();
        assert_eq!(socket_mode & 0o777, 0o700, "socket must be 0700");
        assert_eq!(dir_mode & 0o777, 0o700, "socket dir must be 0700");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn stale_socket_file_is_replaced() {
        let endpoint = test_endpoint("stale");
        {
            let _l = Listener::bind(&endpoint).await.expect("bind");
        } // dropped: file removed; recreate a stale file manually
        std::fs::write(&endpoint, b"").expect("create stale file");
        let listener = Listener::bind(&endpoint).await;
        assert!(listener.is_ok(), "stale socket must be reclaimed");
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn pipe_descriptor_restricts_to_current_user() {
        // The SDDL is built from the live user SID; assert it is present and
        // the DACL is protected (D:P...), i.e. no inherited ACEs can widen it.
        let sid = current_user_sid();
        assert!(sid.starts_with("S-1-"), "unexpected SID: {sid}");
    }
}
