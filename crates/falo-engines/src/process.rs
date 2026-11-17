//! Avvio dei motori come processi figli, con lettura dello stato da stdout.

use std::path::Path;
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

use crate::moonlight::{EngineEvent, parse_event};

pub struct EngineProcess {
    child: Child,
    pub events: mpsc::Receiver<EngineEvent>,
}

impl EngineProcess {
    /// Lancia `bin args…`; ogni riga JSON su stdout diventa un `EngineEvent`.
    pub fn spawn(bin: &Path, args: &[String]) -> std::io::Result<Self> {
        let mut cmd = Command::new(bin);
        cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::null()).kill_on_drop(true);
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        let mut child = cmd.spawn()?;
        let (tx, events) = mpsc::channel(32);
        if let Some(stdout) = child.stdout.take() {
            tokio::spawn(async move {
                let mut lines = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if let Some(ev) = parse_event(&line)
                        && tx.send(ev).await.is_err()
                    {
                        break;
                    }
                }
            });
        }
        Ok(Self { child, events })
    }

    pub async fn stop(&mut self) -> std::io::Result<()> {
        self.child.kill().await
    }

    pub async fn wait(&mut self) -> std::io::Result<std::process::ExitStatus> {
        self.child.wait().await
    }
}
