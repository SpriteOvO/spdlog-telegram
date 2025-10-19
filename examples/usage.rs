use std::{env, error::Error as StdError, process, sync::Arc};

use spdlog::prelude::*;
use spdlog_telegram::{Recipient, TelegramSink};

fn main() {
    let bot_token = env::var("BOT_TOKEN").unwrap_or_else(|_| {
        error!("env var `BOT_TOKEN` is not set");
        process::exit(1);
    });

    let Some(recipient) = env::args().nth(1).map(|input| {
        input
            .parse::<i64>()
            .map_or_else(|_| Recipient::username(input), Recipient::chat_id)
    }) else {
        error!("invalid cli argument. usage: `usage <chat_id | @username>`");
        process::exit(1);
    };

    if let Err(err) = setup_logger(bot_token, recipient) {
        error!("failed to setup logger: {err}");
        process::exit(1);
    }

    trace!("this will only go to stdout");
    info!("this will only go to stdout");
    warn!("this will go to both stderr and Telegram without notification sound");
    error!("this will go to both stderr and Telegram with notification sound");
}

fn setup_logger(bot_token: String, recipient: Recipient) -> Result<(), Box<dyn StdError>> {
    let sink = Arc::new(
        TelegramSink::builder()
            .bot_token(bot_token)
            .recipient(recipient)
            // Notification (sound)
            //  - enabled for logs with level: critical, error;
            //  - disabled for logs with level: warn, info, debug, trace.
            .silence(LevelFilter::MoreVerboseEqual(Level::Warn))
            // We don't care logs with level: info, debug, trace.
            .level_filter(LevelFilter::MoreSevereEqual(Level::Warn))
            .build()?,
    );
    let logger = spdlog::default_logger().fork_with(|logger| {
        logger.set_level_filter(LevelFilter::All);
        logger.sinks_mut().push(sink);
        // Now the new logger has 3 sinks: stdout + stderr + Telegram
        //                                 ^^^^^^^^^^^^^^^
        //                                 forked from the default logger
        Ok(())
    })?;
    spdlog::set_default_logger(logger);
    Ok(())
}
