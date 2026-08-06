//! The backend or processing thread of the interactive app.
//!
//! This is the module for the backend or processing thread used by the
//! interactive version of the app (i.e., either TUI or GUI).
//!
//! This is so that the UI is more responsive and doesn't seem to lock up
//! when there's a long operation.
//!
//! This is not used in the CLI, which uses the main thread for everything,
//! since it is not interactive.

use std::{
    io,
    sync::mpsc,
    thread::{self, JoinHandle},
};

const BACKEND_THREAD_NAME: &str = "TRT-Backend";

/// Represents the state of the app's backend.
pub(crate) struct BackendState {
    /// The connection to (presumably) the frontend.
    conn: FrontendConnection,
}

impl BackendState {
    /// Create a new backend state and its connection, without running it.
    pub(crate) fn new() -> (Self, BackendConnection) {
        let mes = mpsc::channel::<BackendMessage>();
        let rep = mpsc::channel::<BackendReply>();

        let fconn = FrontendConnection {
            tx: rep.0,
            rx: mes.1,
        };

        let bconn = BackendConnection {
            tx: mes.0,
            rx: rep.1,
        };

        let state = Self { conn: fconn };

        (state, bconn)
    }

    /// Create and start up a new backend in a separate thread.
    ///
    /// # Errors
    /// Errors if the backend thread could not be made.
    pub(crate) fn spawn() -> io::Result<BackendHandle> {
        let (state, connection) = Self::new();
        let builder = thread::Builder::new().name(BACKEND_THREAD_NAME.to_string());
        let join_handle = builder.spawn(move || state.run())?;
        Ok(BackendHandle {
            connection,
            join_handle,
        })
    }

    /// Run this backend in the current thread.
    ///
    /// Don't forget to run it in a separate thread from the frontend thread!
    ///
    /// Also consider using [`Self::spawn`] instead of [`Self::run`].
    pub(crate) fn run(self) {
        for mes in self.conn.rx.iter() {
            let rep = mes.process();
            let Ok(()) = self.conn.tx.send(rep) else {
                return;
            };
        }
    }
}

/// Represents a bidirectional connection to (presumably) the frontend, held by
/// the backend.
struct FrontendConnection {
    /// Sender of replies destined to (presumably) the frontend.
    tx: mpsc::Sender<BackendReply>,
    /// Receiver of messages received from (presumably) the frontend.
    rx: mpsc::Receiver<BackendMessage>,
}

/// Represents a bidirectional connection to the backend (presumably held by
/// the frontend).
///
/// You get this from [`AppBackend::new`].
#[derive(Debug)]
pub(crate) struct BackendConnection {
    /// Sender of messages destined to the backend.
    pub(crate) tx: mpsc::Sender<BackendMessage>,
    /// Receiver of replies received from the backend.
    pub(crate) rx: mpsc::Receiver<BackendReply>,
}

/// A struct containing the connection and [`JoinHandle`] to an active, running
/// backend.
pub(crate) struct BackendHandle {
    /// The bidirectional connection to the backend.
    pub(crate) connection: BackendConnection,
    /// The [`JoinHandle`] to the backend thread.
    pub(crate) join_handle: JoinHandle<()>,
}

/// Represents a message destined to the backend.
pub(crate) enum BackendMessage {
    /// An message to test the backend connection.
    ///
    /// If responsive, the backend should reply with [`BackendReply::Pong`] with
    /// the specified ping ID.
    Ping { ping_id: u64 },
}

impl BackendMessage {
    /// Process this message, returning a reply.
    ///
    /// This function is to be called in the backend.
    fn process(self) -> BackendReply {
        match self {
            Self::Ping { ping_id } => BackendReply::Pong { ping_id },
        }
    }
}

/// Represents a reply from the backend.
pub(crate) enum BackendReply {
    /// The reply to the [`BackendMessage::Ping`] message.
    Pong { ping_id: u64 },
}
