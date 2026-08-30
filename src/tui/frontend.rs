//! The frontend: A wrapper around the UI state.

use core::{ops::ControlFlow, time::Duration};
use std::{
    io,
    path::PathBuf,
    sync::mpsc::{self, TryRecvError},
    time::Instant,
};

use ratatui::crossterm;

use crate::{
    backend::{BackendConnection, BackendRequest, BackendResponse},
    cli::clap::TuiArguments,
    consts::backend::{EMSG_BACKEND_CONNECTION_BROKE, EMSG_BACKEND_UNRESPONSIVE},
    paths,
    tui::scenes::Scene,
};

/// Represents the state for the TUI app's frontend.
#[derive(Debug)]
pub(in crate::tui) struct AppFrontend {
    /// The folder of the file explorer if we return to it later.
    pub(in crate::tui) ret_path: PathBuf,
    /// The connection to the backend.
    pub(in crate::tui) conn: BackendConnection,
    /// The currently-displayed UI scene.
    pub(in crate::tui) scene: Scene,
}

impl AppFrontend {
    /// The maximum backend roundtrip time when initializing the frontend.
    const MAX_INIT_BACKEND_RTT: Duration = Duration::from_secs(5);
    /// The maximum amount of time to wait for input events.
    const EVENT_POLL_DURATION: Duration = Duration::from_millis(100);

    /// Creates a new frontend instance. Returns an error if the backend is
    /// unresponsive.
    pub(in crate::tui) fn new(args: TuiArguments, conn: BackendConnection) -> io::Result<Self> {
        Self::check_backend_connection(&conn)?;

        let explorer_path = args.path.clone().unwrap_or_else(paths::get_initial_path);
        let scene = Scene::new(&explorer_path);

        Ok(Self {
            ret_path: explorer_path,
            conn,
            scene,
        })
    }

    /// Checks whether or not the given backend connection responds to pings
    /// in a reasonable amount of time.
    fn check_backend_connection(conn: &BackendConnection) -> io::Result<()> {
        let id: u64 = rand::random();
        let mes = BackendRequest::Ping { ping_id: id };
        if let Err(e) = conn.tx.send(mes) {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, e));
        }

        let deadline = Instant::now() + Self::MAX_INIT_BACKEND_RTT;
        loop {
            let reply = match conn.rx.recv_timeout(Self::MAX_INIT_BACKEND_RTT) {
                Ok(r) => r,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        EMSG_BACKEND_CONNECTION_BROKE,
                    ));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        EMSG_BACKEND_UNRESPONSIVE,
                    ));
                }
            };

            if Instant::now().checked_duration_since(deadline).is_some() {
                // Deadline is in the past
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    EMSG_BACKEND_UNRESPONSIVE,
                ));
            }

            if let BackendResponse::Pong { ping_id } = reply
                && ping_id == id
            {
                return Ok(());
            }
        }
    }

    /// Run this frontend given a ratatui terminal.
    pub(in crate::tui) fn run(
        &mut self,
        terminal: &mut ratatui::DefaultTerminal,
    ) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.scene.render(f))?;

            if let ControlFlow::Break(res) = self.handle_events(terminal) {
                return res;
            }

            self.handle_replies()?;
        }
    }

    /// Handle crossterm (console) events.
    fn handle_events(
        &mut self,
        terminal: &ratatui::DefaultTerminal,
    ) -> ControlFlow<io::Result<()>> {
        match crossterm::event::poll(Self::EVENT_POLL_DURATION) {
            Ok(true) => (),
            Ok(false) => return ControlFlow::Continue(()),
            Err(e) => return ControlFlow::Break(Err(e)),
        }

        let event = match crossterm::event::read() {
            Ok(ev) => ev,
            Err(e) => return ControlFlow::Break(Err(e)),
        };

        self.scene
            .handle_event(event, terminal, &mut self.conn, &mut self.ret_path)
            .map_break(|()| Ok(()))?;

        ControlFlow::Continue(())
    }

    /// Handle replies from the backend.
    fn handle_replies(&mut self) -> io::Result<()> {
        loop {
            let reply = match self.conn.rx.try_recv() {
                Ok(rep) => rep,
                Err(TryRecvError::Empty) => return Ok(()),
                Err(TryRecvError::Disconnected) => {
                    return Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        EMSG_BACKEND_CONNECTION_BROKE,
                    ));
                }
            };

            self.scene.handle_response(reply, &self.conn.tx);
        }
    }
}
