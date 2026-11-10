//! The portable backend: a work-stealing tokio runtime, one task per connection.

use std::future::Future;
use std::io;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;

use crate::{ConnState, Flow, Lobby};

/// How long to keep draining a closing connection so our last bytes (a disconnect message) are
/// not destroyed by a reset caused by unread data from the peer.
const LINGER: Duration = Duration::from_millis(250);

/// Accepts connections until `shutdown` completes.
pub async fn serve(
    listener: TcpListener,
    lobby: Arc<Lobby>,
    shutdown: impl Future<Output = ()>,
) -> io::Result<()> {
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let (stream, _) = match accepted {
                    Ok(a) => a,
                    // Running out of file descriptors or a connection that died in the backlog
                    // must not take the server down.
                    Err(e) => {
                        tracing::warn!("accept failed: {e}");
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        continue;
                    }
                };
                tokio::spawn(handle_connection(stream, lobby.clone()));
            }
            () = &mut shutdown => return Ok(()),
        }
    }
}

/// Runs one connection to completion.
pub async fn handle_connection(mut stream: TcpStream, lobby: Arc<Lobby>) {
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
    loop {
        conn.session.recv_buffer().reserve(4096);
        let read = timeout(read_timeout, stream.read_buf(conn.session.recv_buffer())).await;
        let n = match read {
            Ok(result) => result?,
            Err(_) => {
                lobby.stats.timeouts.fetch_add(1, Ordering::Relaxed);
                return Ok(());
            }
        };
        if n == 0 {
            return Ok(());
        }
        conn.session.commit();

        let mut flow = lobby.drive(conn);
        loop {
            flush(stream, conn).await?;
            match flow {
                Ok(Flow::Continue) => break,
                Ok(Flow::Close) | Err(_) => return linger(stream).await,
                Ok(Flow::Authenticate(request)) => {
                    // The authenticator answers on a channel, so the runtime it uses is irrelevant.
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
    let n = conn.session.pending_output().len();
    if n > 0 {
        stream.write_all(conn.session.pending_output()).await?;
        conn.session.consume_output(n);
    }
    Ok(())
}

/// Half-close, then swallow whatever the peer still sends until it closes or time runs out.
async fn linger(stream: &mut TcpStream) -> io::Result<()> {
    stream.shutdown().await?;
    let mut sink = [0u8; 1024];
    let _ =
        timeout(LINGER, async { while matches!(stream.read(&mut sink).await, Ok(n) if n > 0) {} })
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
