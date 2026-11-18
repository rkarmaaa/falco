//! Avvio dei motori come processi figli, con priorità alzata e lettura dello stato.

use std::path::Path;
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

use crate::moonlight::{EngineEvent, StatsCollector, parse_event};

/// Priorità del processo su Windows (§10 «Durante lo streaming»).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    Normal,
    AboveNormal,
    High,
}

impl Priority {
    #[cfg(windows)]
    fn creation_flag(self) -> u32 {
        match self {
            Self::Normal => 0x0000_0020,      // NORMAL_PRIORITY_CLASS
            Self::AboveNormal => 0x0000_8000, // ABOVE_NORMAL_PRIORITY_CLASS
            Self::High => 0x0000_0080,        // HIGH_PRIORITY_CLASS
        }
    }
}

/// Costruisce il comando: niente console, priorità richiesta, chiusura con il core.
pub fn command(bin: &Path, args: &[String], priority: Priority) -> Command {
    let mut cmd = Command::new(bin);
    cmd.args(args).kill_on_drop(true);
    // Moonlight cerca portable.dat nella cartella di lavoro: impostazioni separate
    if let Some(dir) = bin.parent() {
        cmd.current_dir(dir);
    }
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000 | priority.creation_flag()); // CREATE_NO_WINDOW
    #[cfg(not(windows))]
    let _ = priority;
    cmd
}

pub struct EngineProcess {
    child: Child,
    pub events: mpsc::Receiver<EngineEvent>,
}

fn forward<R: AsyncRead + Unpin + Send + 'static>(reader: R, tx: mpsc::Sender<EngineEvent>) {
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        let mut stats = StatsCollector::default();
        while let Ok(Some(line)) = lines.next_line().await {
            let ev = stats.feed(&line).map(EngineEvent::Summary).or_else(|| parse_event(&line));
            if let Some(ev) = ev
                && tx.send(ev).await.is_err()
            {
                break;
            }
        }
    });
}

impl EngineProcess {
    /// Lancia il motore; le righe di stdout e stderr diventano `EngineEvent`.
    pub fn spawn(bin: &Path, args: &[String], priority: Priority) -> std::io::Result<Self> {
        let mut cmd = command(bin, args, priority);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = cmd.spawn()?;
        let (tx, events) = mpsc::channel(32);
        if let Some(out) = child.stdout.take() {
            forward(out, tx.clone());
        }
        if let Some(err) = child.stderr.take() {
            forward(err, tx);
        }
        Ok(Self { child, events })
    }

    /// Separa processo ed eventi, per attenderli insieme con `select!`.
    pub fn into_parts(self) -> (Child, mpsc::Receiver<EngineEvent>) {
        (self.child, self.events)
    }

    pub fn id(&self) -> Option<u32> {
        self.child.id()
    }

    pub async fn stop(&mut self) -> std::io::Result<()> {
        self.child.kill().await
    }

    pub fn try_finished(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(Some(_)))
    }

    pub async fn wait(&mut self) -> std::io::Result<std::process::ExitStatus> {
        self.child.wait().await
    }
}
