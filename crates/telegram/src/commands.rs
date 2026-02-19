use std::sync::Arc;

use teloxide::{
    dispatching::UpdateHandler,
    prelude::*,
    utils::command::BotCommands,
};
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn};

use common::{EngineCommand, EngineState, TradingMode};

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

/// Dependencies injected into every handler via `dptree`.
#[derive(Clone)]
pub struct BotDeps {
    pub command_tx: mpsc::Sender<EngineCommand>,
    pub engine_state: Arc<RwLock<EngineState>>,
    pub trading_mode: TradingMode,
    pub allowed_user_ids: Arc<Vec<i64>>,
    /// Channel for sending alerts back to the bot (used by Risk Manager).
    pub alert_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<String>>>,
}

/// Telegram bot commands exposed to the operator.
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "ClawBot commands:")]
pub enum Command {
    #[command(description = "Start the trading engine")]
    Start,
    #[command(description = "Stop the trading engine (closes open positions)")]
    Stop,
    #[command(description = "Show engine status and PnL summary")]
    Status,
    #[command(description = "Reset max-drawdown halt")]
    ResetDrawdown,
}

/// Start the Telegram bot in long-polling mode.
pub async fn start_bot(token: String, deps: BotDeps) {
    let bot = Bot::new(token);
    let deps = Arc::new(deps);

    info!("Telegram bot starting (long-polling)");

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![deps])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync>> {
    use dptree::case;

    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(case![Command::Start].endpoint(handle_start))
        .branch(case![Command::Stop].endpoint(handle_stop))
        .branch(case![Command::Status].endpoint(handle_status))
        .branch(case![Command::ResetDrawdown].endpoint(handle_reset_drawdown));

    Update::filter_message()
        .filter_map(|msg: Message| msg.from().map(|u| u.id))
        .filter_async(auth_filter)
        .branch(command_handler)
}

/// Silently drop messages from users not in the allowed list.
async fn auth_filter(user_id: UserId, deps: Arc<BotDeps>) -> bool {
    let uid = user_id.0 as i64;
    let allowed = deps.allowed_user_ids.contains(&uid);
    if !allowed {
        warn!(user_id = uid, "Unauthorized Telegram access attempt");
    }
    allowed
}

async fn handle_start(bot: Bot, msg: Message, deps: Arc<BotDeps>) -> HandlerResult {
    let state = *deps.engine_state.read().await;
    if state == EngineState::Running {
        bot.send_message(msg.chat.id, "Engine is already running.").await?;
    } else {
        let _ = deps.command_tx.send(EngineCommand::Start).await;
        bot.send_message(msg.chat.id, "Engine started.").await?;
    }
    Ok(())
}

async fn handle_stop(bot: Bot, msg: Message, deps: Arc<BotDeps>) -> HandlerResult {
    let state = *deps.engine_state.read().await;
    if state == EngineState::Stopped {
        bot.send_message(msg.chat.id, "Engine is already stopped.").await?;
    } else {
        bot.send_message(msg.chat.id, "Closing open positions and stopping\u{2026}").await?;
        let _ = deps.command_tx.send(EngineCommand::Stop).await;
        bot.send_message(msg.chat.id, "Engine stopped.").await?;
    }
    Ok(())
}

async fn handle_status(bot: Bot, msg: Message, deps: Arc<BotDeps>) -> HandlerResult {
    let state = *deps.engine_state.read().await;
    let mode = deps.trading_mode;
    let text = format!(
        "ClawBot Status\n\
         Engine: {state}\n\
         Mode: {mode}\n\
         (PnL data available via dashboard)"
    );
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

async fn handle_reset_drawdown(bot: Bot, msg: Message, deps: Arc<BotDeps>) -> HandlerResult {
    let state = *deps.engine_state.read().await;
    if state != EngineState::Halted {
        bot.send_message(msg.chat.id, "No active drawdown halt.").await?;
    } else {
        let _ = deps.command_tx.send(EngineCommand::ResetDrawdown).await;
        bot.send_message(msg.chat.id, "Drawdown reset. Engine resuming.").await?;
    }
    Ok(())
}

/// Send a proactive alert to all configured chat IDs.
/// Call this from the Risk Manager event loop.
pub async fn send_alert(bot: &Bot, chat_ids: &[ChatId], message: &str) {
    for &chat_id in chat_ids {
        if let Err(e) = bot.send_message(chat_id, message).await {
            warn!(chat_id = ?chat_id, error = %e, "Failed to send Telegram alert");
        }
    }
}
