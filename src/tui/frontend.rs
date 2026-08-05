//! The frontend: A wrapper around the UI state.

use core::time::Duration;
use std::{io, path::PathBuf, sync::mpsc, time::Instant};

use crate::{
    backend::{BackendConnection, BackendMessage, BackendReply},
    cli::clap::TuiArguments,
    paths,
    tui::ui_state::{ExplorerState, UiState},
};

/// Represents the state for the TUI app's frontend.
pub(in crate::tui) struct AppFrontend {
    /// The folder of the file explorer if we return to it later.
    ret_path: PathBuf,
    /// The connection to the backend.
    conn: BackendConnection,
    /// The state of the UI.
    ui_state: UiState,
}

impl AppFrontend {
    /// The maximum backend roundtrip time when initializing the frontend.
    const MAX_INIT_BACKEND_RTT: Duration = Duration::from_secs(5);

    /// Creates a new frontend instance. Returns an error if the backend is
    /// unresponsive.
    pub(in crate::tui) fn new(args: TuiArguments, conn: BackendConnection) -> io::Result<Self> {
        Self::check_backend_connection(&conn)?;

        let explorer_path = args.path.clone().unwrap_or_else(paths::get_initial_path);
        let ui_state = UiState::Explorer(ExplorerState::new(&explorer_path));

        Ok(Self {
            ret_path: explorer_path,
            conn,
            ui_state,
        })
    }

    /// Checks whether or not the given backend connection responds to pings
    /// in a reasonable amount of time.
    fn check_backend_connection(conn: &BackendConnection) -> io::Result<()> {
        static BACKEND_CONNECTION_BROKE: &str = "Connection to backend broke";
        static BACKEND_UNRESPONSIVE: &str = "Backend is unresponsive";

        let id: u64 = rand::random();
        let mes = BackendMessage::Ping { ping_id: id };
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
                        BACKEND_CONNECTION_BROKE,
                    ));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        BACKEND_UNRESPONSIVE,
                    ));
                }
            };

            if Instant::now().checked_duration_since(deadline).is_some() {
                // Deadline is in the past
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    BACKEND_UNRESPONSIVE,
                ));
            }

            if let BackendReply::Pong { ping_id } = reply
                && ping_id == id
            {
                return Ok(());
            }
        }
    }

    /// Run this frontend given a ratatui terminal.
    pub(in crate::tui) fn run(&mut self, terminal: &mut ratatui::DefaultTerminal) {
        loop {
            todo!("Draw, get events, handle connection, etc.");
        }
    }
}
