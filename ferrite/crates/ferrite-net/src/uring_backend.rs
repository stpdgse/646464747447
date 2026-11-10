//! The thread-per-core backend: one `io_uring` runtime per worker thread, each accepting from its
//! own `SO_REUSEPORT` listener. Connections never migrate between threads, so there is no
//! work-stealing, no cross-thread wakeups and no locks on the connection path.

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use monoio::io::{AsyncReadRent, AsyncWriteRent, AsyncWriteRentExt};
use monoio::net::{TcpListener, TcpStream};
use monoio::time::timeout;
use monoio::{IoUringDriver, RuntimeBuilder};
use socket2::{Domain, Protocol, Socket, Type};

use crate::{ConnState, Flow, Lobby};

const LINGER: Duration = Duration::from_millis(250);
/// How often an idle accept loop checks whether it should stop.
const STOP_POLL: Duration = Duration::from_millis(100);
const READ_BUFFER: usize = 8192;

/// A running set of worker threads.
#[derive(Debug)]
pub struct Server {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    workers: Vec<JoinHandle<io::Result<()>>>,
}

impl Server {
    /// The address actually bound (useful with port 0).
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    /// Stops accepting and waits for the workers. Open connections are closed with the runtimes.
    pub fn shutdown(mut self) -> io::Result<()> {
        self.stop.store(true, Ordering::Release);
        let mut result = Ok(());
        for worker in self.workers.drain(..) {
            match worker.join() {
                Ok(Ok(())) => {}
                Ok(Err(e)) => result = Err(e),
                Err(_) => result = Err(io::Error::other("a worker thread panicked")),
            }
        }
        result
    }

    /// Blocks until the workers exit (normally never).
    pub fn wait(mut self) -> io::Result<()> {
        let mut result = Ok(());
        for worker in self.workers.drain(..) {
            match worker.join() {
                Ok(Ok(())) => {}
                Ok(Err(e)) => result = Err(e),
                Err(_) => result = Err(io::Error::other("a worker thread panicked")),
            }
        }
        result
    }
}

/// Whether this kernel lets us create an `io_uring` runtime (it can be disabled by sysctl or a
/// sandbox policy).
pub fn is_available() -> bool {
    RuntimeBuilder::<IoUringDriver>::new().with_entries(8).build().is_ok()
}

fn bind(addr: SocketAddr) -> io::Result<std::net::TcpListener> {
    let socket = Socket::new(Domain::for_address(addr), Type::STREAM, Some(Protocol::TCP))?;
    socket.set_reuse_address(true)?;
    socket.set_reuse_port(true)?;
    socket.bind(&addr.into())?;
    socket.listen(4096)?;
    socket.set_nonblocking(true)?;
    Ok(socket.into())
}

/// Starts `threads` workers listening on `addr`. Fails if `io_uring` is unavailable.
pub fn serve(addr: SocketAddr, threads: usize, lobby: Arc<Lobby>) -> io::Result<Server> {
    assert!(threads > 0, "at least one worker thread is required");
    let first = bind(addr)?;
    let addr = first.local_addr()?;
    let mut listeners = vec![first];
    for _ in 1..threads {
        listeners.push(bind(addr)?);
    }

    let stop = Arc::new(AtomicBool::new(false));
    // Surface "io_uring is not available" to the caller instead of inside a worker.
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<io::Result<()>>();
    let workers = listeners
        .into_iter()
        .enumerate()
        .map(|(i, listener)| {
            let (lobby, stop, ready) = (lobby.clone(), stop.clone(), ready_tx.clone());
            std::thread::Builder::new()
                .name(format!("ferrite-uring-{i}"))
                .spawn(move || worker(listener, lobby, stop, ready))
        })
        .collect::<io::Result<Vec<_>>>()?;
    drop(ready_tx);

    for _ in 0..threads {
        if let Ok(Err(e)) = ready_rx.recv() {
            stop.store(true, Ordering::Release);
            return Err(e);
        }
    }
    Ok(Server { addr, stop, workers })
}

fn worker(
    listener: std::net::TcpListener,
    lobby: Arc<Lobby>,
    stop: Arc<AtomicBool>,
    ready: std::sync::mpsc::Sender<io::Result<()>>,
) -> io::Result<()> {
    let runtime = RuntimeBuilder::<IoUringDriver>::new().with_entries(4096).enable_timer().build();
    let mut runtime = match runtime {
        Ok(rt) => rt,
        Err(e) => {
            let _ = ready.send(Err(io::Error::new(e.kind(), format!("io_uring unavailable: {e}"))));
            return Err(e);
        }
    };
    let _ = ready.send(Ok(()));
    runtime.block_on(async move {
        let listener = TcpListener::from_std(listener)?;
        while !stop.load(Ordering::Acquire) {
            match timeout(STOP_POLL, listener.accept()).await {
                Ok(Ok((stream, _))) => {
                    monoio::spawn(handle_connection(stream, lobby.clone()));
                }
                Ok(Err(e)) => {
                    tracing::warn!("accept failed: {e}");
                    monoio::time::sleep(Duration::from_millis(10)).await;
                }
                Err(_) => {} // timed out: re-check the stop flag
            }
        }
        Ok(())
    })
}

async fn handle_connection(mut stream: TcpStream, lobby: Arc<Lobby>) {
    let _ = stream.set_nodelay(true);
    let mut conn = lobby.connect(stream.peer_addr().ok().map(|a| a.ip()));
    match run(&mut stream, &lobby, &mut conn).await {
        Ok(()) => {}
        Err(e) if is_peer_gone(&e) => {}
        Err(e) => tracing::debug!("connection ended with error: {e}"),
    }
    lobby.disconnect(&mut conn);
}

async fn run(stream: &mut TcpStream, lobby: &Lobby, conn: &mut ConnState) -> io::Result<()> {
    let read_timeout = lobby.config().read_timeout;
    let mut buf = Vec::with_capacity(READ_BUFFER);
    loop {
        // io_uring owns the buffer while the read is in flight, so it travels by value.
        let (result, returned) = match timeout(read_timeout, stream.read(buf)).await {
            Ok(r) => r,
            Err(_) => {
                lobby.stats.timeouts.fetch_add(1, Ordering::Relaxed);
                return Ok(());
            }
        };
        buf = returned;
        let n = result?;
        if n == 0 {
            return Ok(());
        }
        conn.session.feed(&buf[..n]);
        buf.clear();

        let mut flow = lobby.drive(conn);
        loop {
            flush(stream, conn).await?;
            match flow {
                Ok(Flow::Continue) => break,
                Ok(Flow::Close) | Err(_) => return linger(stream).await,
                Ok(Flow::Authenticate(request)) => {
                    // tokio's oneshot channel is runtime-agnostic: it only needs a waker.
                    let answer = timeout(lobby.auth_timeout(), lobby.start_authentication(request))
                        .await
                        .ok()
                        .and_then(Result::ok);
                    flow = lobby.finish_authentication(conn, answer);
                }
            }
        }
    }
}

async fn flush(stream: &mut TcpStream, conn: &mut ConnState) -> io::Result<()> {
    if conn.session.pending_output().is_empty() {
        return Ok(());
    }
    let out = conn.session.take_output().freeze();
    let (result, _) = stream.write_all(out).await;
    result.map(|_| ())
}

async fn linger(stream: &mut TcpStream) -> io::Result<()> {
    stream.shutdown().await?;
    let _ = timeout(LINGER, async {
        let mut sink = Vec::with_capacity(1024);
        loop {
            let (result, returned) = stream.read(sink).await;
            sink = returned;
            sink.clear();
            if !matches!(result, Ok(n) if n > 0) {
                break;
            }
        }
    })
    .await;
    Ok(())
}

fn is_peer_gone(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::ConnectionReset
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::BrokenPipe
            | io::ErrorKind::UnexpectedEof
    )
}
