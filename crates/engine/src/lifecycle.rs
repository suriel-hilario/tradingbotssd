use std::sync::Arc;

use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{info, warn};

use common::{EngineCommand, EngineState, MarketEvent};

use crate::binance::BinanceStream;

/// Cloneable handle passed to other crates (Telegram, API).
#[derive(Clone)]
pub struct EngineHandle {
    command_tx: mpsc::Sender<EngineCommand>,
    state: Arc<RwLock<EngineState>>,
    market_tx: broadcast::Sender<MarketEvent>,
}

impl EngineHandle {
    pub async fn send(&self, cmd: EngineCommand) {
        let _ = self.command_tx.send(cmd).await;
    }

    pub async fn state(&self) -> EngineState {
        *self.state.read().await
    }

    /// Subscribe to the market event broadcast.
    pub fn subscribe_market(&self) -> broadcast::Receiver<MarketEvent> {
        self.market_tx.subscribe()
    }
}

/// The main engine: manages WebSocket stream lifecycle and command processing.
pub struct Engine {
    pairs: Vec<String>,
    state: Arc<RwLock<EngineState>>,
    market_tx: broadcast::Sender<MarketEvent>,
    command_rx: mpsc::Receiver<EngineCommand>,
    #[allow(dead_code)] // kept to prevent channel close
    command_tx: mpsc::Sender<EngineCommand>,
    /// Hook called after every reconnect to trigger a position audit.
    on_reconnect: Option<Box<dyn Fn() + Send + Sync>>,
}

impl Engine {
    pub fn new(pairs: Vec<String>) -> (Self, EngineHandle) {
        let (command_tx, command_rx) = mpsc::channel(32);
        let (market_tx, _) = broadcast::channel(1024);
        let state = Arc::new(RwLock::new(EngineState::Stopped));

        let handle = EngineHandle {
            command_tx: command_tx.clone(),
            state: state.clone(),
            market_tx: market_tx.clone(),
        };

        let engine = Engine {
            pairs,
            state,
            market_tx,
            command_rx,
            command_tx,
            on_reconnect: None,
        };

        (engine, handle)
    }

    pub fn on_reconnect<F: Fn() + Send + Sync + 'static>(&mut self, f: F) {
        self.on_reconnect = Some(Box::new(f));
    }

    /// Run the engine. This task drives stream spawning and command processing.
    /// Call from `tokio::spawn`.
    pub async fn run(mut self) {
        info!("Engine initialized in Stopped state. Waiting for Start command.");

        let mut stream_handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();

        loop {
            match self.command_rx.recv().await {
                Some(EngineCommand::Start) => {
                    let current = *self.state.read().await;
                    if current == EngineState::Running {
                        info!("Engine already running");
                        continue;
                    }

                    info!(pairs = ?self.pairs, "Starting market data streams");
                    *self.state.write().await = EngineState::Running;

                    // Spawn one WebSocket stream per pair
                    for pair in &self.pairs {
                        let stream = BinanceStream::new(pair.clone(), self.market_tx.clone());
                        let handle = tokio::spawn(stream.run());
                        stream_handles.push(handle);
                    }
                }

                Some(EngineCommand::Stop) => {
                    info!("Engine stopping — aborting stream tasks");
                    *self.state.write().await = EngineState::Stopped;
                    for h in stream_handles.drain(..) {
                        h.abort();
                    }
                }

                Some(EngineCommand::Pause) => {
                    let current = *self.state.read().await;
                    if current == EngineState::Running {
                        info!("Engine paused — streams continue, signals suppressed");
                        *self.state.write().await = EngineState::Paused;
                    }
                }

                Some(EngineCommand::Resume) => {
                    let current = *self.state.read().await;
                    if current == EngineState::Paused {
                        info!("Engine resumed");
                        *self.state.write().await = EngineState::Running;
                    }
                }

                Some(EngineCommand::ResetDrawdown) => {
                    let current = *self.state.read().await;
                    if current == EngineState::Halted {
                        info!("Drawdown reset — engine resuming");
                        *self.state.write().await = EngineState::Running;
                    } else {
                        warn!("ResetDrawdown received but engine is not halted");
                    }
                }

                None => {
                    warn!("Engine command channel closed — shutting down");
                    break;
                }
            }
        }
    }
}
