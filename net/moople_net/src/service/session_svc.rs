use std::{fmt::Debug, io, marker::PhantomData, time::Duration};

use futures::{Stream, StreamExt};
use moople_packet::NetError;
use tokio::{
    net::{TcpListener, TcpStream, ToSocketAddrs},
    sync::mpsc,
};
use tokio_util::sync::CancellationToken;

use crate::{codec::handshake::Handshake, service::handler::SessionError, MapleSession};

use super::{
    framed_pipe::{framed_pipe, FramedPipeReceiver, FramedPipeSender},
    handler::{MakeServerSessionHandler, MapleServerSessionHandler, MapleSessionHandler},
    HandshakeGenerator,
};

#[derive(Debug, Clone)]
pub struct SharedSessionHandle {
    pub ct: CancellationToken,
    pub tx: FramedPipeSender,
}

impl SharedSessionHandle {
    pub fn new() -> (Self, FramedPipeReceiver) {
        let (tx, rx) = framed_pipe(8 * 1024, 128);
        (
            Self {
                ct: CancellationToken::new(),
                tx,
            },
            rx,
        )
    }
}

#[derive(Debug)]
pub struct MapleSessionHandle<H: MapleSessionHandler> {
    pub handle: tokio::task::JoinHandle<Result<(), SessionError<H::Error>>>,
    _handler: PhantomData<H>,
}

impl<H> MapleSessionHandle<H>
where
    H: MapleSessionHandler + Send,
{
    /*pub fn cancel(&mut self) {
        self.session_handle.ct.cancel();
    }*/

    pub fn is_running(&self) -> bool {
        !self.handle.is_finished()
    }

    async fn exec_server_session(
        mut session: MapleSession<H::Transport>,
        mut handler: H,
        session_handle: SharedSessionHandle,
        mut session_rx: FramedPipeReceiver
    ) -> Result<(), SessionError<H::Error>>
    where
        H: MapleServerSessionHandler,
        H::Transport: Unpin,
    {
        let mut ping_interval = tokio::time::interval(H::get_ping_interval());
        ping_interval.tick().await;

        loop {
            //TODO might need some micro-optimization to ensure no future gets stalled
            tokio::select! {
                biased;
                // Handle next incoming packet
                p = session.read_packet() => {
                    let res = match p {
                        Ok(p) => handler.handle_packet(p, &mut session).await,
                        Err(net_err) => Err(SessionError::Net(net_err))
                    };


                    // If there's an error handle it
                    if let Err(err) = res {
                        log::info!("Err: {:?}", err);
                        match err {
                            SessionError::Net(NetError::IO(err)) if err.kind() == std::io::ErrorKind::UnexpectedEof  => {
                                log::info!("Client disconnected");
                                break;
                            },
                            SessionError::Net(NetError::Migrated) => {
                                log::info!("Session migrated");
                                handler.finish(true).await?;
                                // Socket has to be kept open cause the client doesn't support
                                // reading a packet when the socket is closed
                                // TODO: make this configurable
                                tokio::time::sleep(Duration::from_millis(7500)).await;
                                break;
                            },
                            _ => {}
                        };
                    }
                },
                _ = ping_interval.tick() => {
                    let ping_packet = handler.get_ping_packet().map_err(SessionError::Session)?;
                    log::info!("Sending ping packet: {:?}", ping_packet.data);
                    session.send_raw_packet(&ping_packet.data).await?;
                },
                //Handle external Session packets
                p = session_rx.next() => {
                    // note tx is never dropped, so there'll be always a packet here
                    let p = p.expect("Session packet");
                    session.send_raw_packet(&p).await?;
                },
                p = handler.poll_broadcast() => {
                    let p = p.map_err(SessionError::Session)?.expect("Must contain packet");
                    session.send_raw_packet(&p.data).await?;
                },
                _ = session_handle.ct.cancelled() => {
                    break;
                },

            };
        }

        session.close().await?;

        // Normal cancellation by timeout or cancellation
        // TODO: handle panic and gracefully shutdown the session(for example write data to db and other stuff)
        Ok(())
    }

    pub fn spawn_server_session<M>(
        io: M::Transport,
        mut mk: M,
        handshake: Handshake,
    ) -> Result<Self, SessionError<M::Error>>
    where
        M: MakeServerSessionHandler<Handler = H, Transport = H::Transport, Error = H::Error>
            + Send
            + 'static,
        H: MapleServerSessionHandler + Send + 'static,
        H::Transport: Unpin + Send + 'static,
        H::Error: Send + 'static,
    {
        let handle = tokio::spawn(async move {
            let res = async move {
                let mut session = MapleSession::initialize_server_session(io, handshake).await?;

                let (sess_handle, sess_rx) = SharedSessionHandle::new();
                let handler = mk
                    .make_handler(&mut session, sess_handle.clone())
                    .await
                    .map_err(SessionError::Session)?;

                let res =
                    Self::exec_server_session(session, handler, sess_handle, sess_rx).await;
                if let Err(ref err) = res {
                    log::info!("Session exited with error: {:?}", err);
                }

                Ok(())
            };

            let res = res.await;
            if let Err(ref err) = res {
                log::error!("Session error: {:?}", err);
            }

            res
        });

        Ok(MapleSessionHandle {
            handle,
            _handler: PhantomData,
        })
    }
}

#[derive(Debug)]
pub struct MapleServer<MH, H>
where
    MH: MakeServerSessionHandler,
{
    handshake_gen: H,
    make_handler: MH,
    handles: Vec<MapleSessionHandle<MH::Handler>>,
}

impl<MH, H> MapleServer<MH, H>
where
    H: HandshakeGenerator,
    MH: MakeServerSessionHandler,
    MH::Handler: Send,
{
    pub fn new(handshake_gen: H, make_handler: MH) -> Self {
        Self {
            handshake_gen,
            make_handler,
            handles: Vec::new(),
        }
    }

    fn remove_closed_handles(&mut self) {
        self.handles.retain(|handle| handle.is_running());
    }

    fn handle_incoming(&mut self, io: MH::Transport) -> Result<(), SessionError<MH::Error>>
    where
        MH: Send + Clone + 'static,
        MH::Error: From<io::Error> + Send + 'static,
        MH::Handler: Send + 'static,
        MH::Transport: Send + Unpin + 'static,
    {
        let handshake = self.handshake_gen.generate_handshake();
        let handle =
            MapleSessionHandle::spawn_server_session(io, self.make_handler.clone(), handshake)?;
        // TODO: there should be an upper limit for active connections
        // cleaning closed connection should operate on Vec<Option<Handle>> probably
        // so a new conneciton just has to find a gap
        // If the last insert/clean index is stored performance should be good
        self.remove_closed_handles();
        self.handles.push(handle);

        Ok(())
    }

    pub async fn run<S>(&mut self, mut io: S) -> Result<(), SessionError<MH::Error>>
    where
        MH: Send + Clone + 'static,
        MH::Error: From<io::Error> + Send + 'static,
        MH::Handler: Send + 'static,
        MH::Transport: Send + Unpin + 'static,
        S: Stream<Item = std::io::Result<MH::Transport>> + Unpin,
    {
        while let Some(io) = io.next().await {
            let io = io.map_err(NetError::IO)?;
            self.handle_incoming(io)?;
        }

        Ok(())
    }
}

impl<MH, H> MapleServer<MH, H>
where
    H: HandshakeGenerator,
    MH::Error: From<io::Error> + Send + 'static,
    MH::Handler: Send + 'static,
    MH::Transport: Send + Unpin + 'static,
    MH: MakeServerSessionHandler<Transport = TcpStream> + Send + Clone + 'static,
    MH::Error: From<io::Error> + Send + 'static,
{
    pub async fn serve_tcp(
        &mut self,
        addr: impl ToSocketAddrs,
    ) -> Result<(), SessionError<MH::Error>> {
        let listener = TcpListener::bind(addr).await.map_err(NetError::IO)?;

        loop {
            let (io, _) = listener.accept().await.map_err(NetError::IO)?;
            self.handle_incoming(io)?;
        }
    }
}
