//! Sends logs to Telegram, based on [spdlog-rs].
//!
//! This crate provides a sink [`TelegramSink`] which sends logs to Telegram
//! recipients via Telegram Bot API.
//!
//! ## Examples
//!
//! See directory [./examples].
//!
//! [spdlog-rs]: https://crates.io/crates/spdlog-rs
//! [./examples]: https://github.com/SpriteOvO/spdlog-telegram/tree/main/examples

#![warn(missing_docs)]

mod error;
mod recipient;
mod request;

use std::{convert::Infallible, sync::atomic::Ordering};

use atomic::Atomic;
pub use error::{Error, Result};
pub use recipient::Recipient;
use request::Requester;
use spdlog::{
    ErrorHandler, Record, StringBuf,
    formatter::{Formatter, FormatterContext, PatternFormatter, pattern},
    prelude::*,
    sink::{GetSinkProp, Sink, SinkProp},
};
use url::Url;

/// A sink with a Telegram recipient as the target via Telegram Bot API.
///
/// This sink involves network operations. If you don't want it to block the
/// thread, you may want to use it in combination with [`AsyncPoolSink`].
///
/// [`AsyncPoolSink`]: https://docs.rs/spdlog-rs/0.5.1/spdlog/sink/struct.AsyncPoolSink.html
pub struct TelegramSink {
    prop: SinkProp,
    silence: Atomic<LevelFilter>,
    requester: Requester,
}

impl TelegramSink {
    /// Gets a builder of `TelegramSink` with default parameters:
    ///
    /// | Parameter         | Default Value                                                                           |
    /// |-------------------|-----------------------------------------------------------------------------------------|
    /// | [level_filter]    | `All`                                                                                   |
    /// | [formatter]       | pattern `"#log #{level} {payload} {kv}\n@{source}"` or `"#log #{level} {payload} {kv}"` |
    /// | [error_handler]   | [`ErrorHandler::default()`]                                                             |
    /// |                   |                                                                                         |
    /// | [server_url]      | `"https://api.telegram.org"`                                                            |
    /// | [bot_token]       | *must be specified*                                                                     |
    /// | [recipient]       | *must be specified*                                                                     |
    /// | [silence]         | `Off`                                                                                   |
    ///
    /// [level_filter]: TelegramSinkBuilder::level_filter
    /// [formatter]: TelegramSinkBuilder::formatter
    /// [error_handler]: TelegramSinkBuilder::error_handler
    /// [`ErrorHandler::default()`]: spdlog::error::ErrorHandler::default()
    /// [server_url]: TelegramSinkBuilder::server_url
    /// [bot_token]: TelegramSinkBuilder::bot_token
    /// [recipient]: TelegramSinkBuilder::recipient
    /// [silence]: TelegramSinkBuilder::silence
    #[must_use]
    pub fn builder() -> TelegramSinkBuilder<(), ()> {
        let prop = SinkProp::default();
        if spdlog::source_location_current!().is_some() {
            prop.set_formatter(PatternFormatter::new(pattern!(
                "#log #{level} {payload} {kv}\n@{source}"
            )));
        } else {
            prop.set_formatter(PatternFormatter::new(pattern!(
                "#log #{level} {payload} {kv}"
            )))
        };
        TelegramSinkBuilder {
            prop,
            server_url: None,
            bot_token: (),
            recipient: (),
            silence: LevelFilter::Off,
        }
    }

    /// Gets the silence level filter.
    #[must_use]
    pub fn silence(&self) -> LevelFilter {
        self.silence.load(Ordering::Relaxed)
    }

    /// Sets the silence level filter.
    ///
    /// Logs with level matching the filter will be sent with
    /// `disable_notification` set to `true`.
    pub fn set_silence(&self, silent_if: LevelFilter) {
        self.silence.store(silent_if, Ordering::Relaxed);
    }
}

impl GetSinkProp for TelegramSink {
    fn prop(&self) -> &SinkProp {
        &self.prop
    }
}

impl Sink for TelegramSink {
    fn log(&self, record: &Record) -> spdlog::Result<()> {
        let mut string_buf = StringBuf::new();
        let mut ctx = FormatterContext::new();
        self.prop
            .formatter()
            .format(record, &mut string_buf, &mut ctx)?;

        self.requester
            .send_log(string_buf, self.silence().test(record.level()))
            .map_err(|err| spdlog::Error::Downstream(err.into()))?;
        Ok(())
    }

    fn flush(&self) -> spdlog::Result<()> {
        Ok(())
    }
}

/// #
///
/// # Note
///
/// The generics here are designed to check for required fields at compile time,
/// users should not specify them manually and/or depend on them. If the generic
/// concrete types or the number of generic types are changed in the future, it
/// may not be considered as a breaking change.
pub struct TelegramSinkBuilder<ArgT, ArgR> {
    prop: SinkProp,
    server_url: Option<Url>,
    bot_token: ArgT,
    recipient: ArgR,
    silence: LevelFilter,
}

impl<ArgT, ArgD> TelegramSinkBuilder<ArgT, ArgD> {
    /// Specifies the Telegram Bot API server URL.
    ///
    /// See [Telegram Bot API: Using a Local Bot API Server][local-srv].
    ///
    /// This parameter is **optional**.
    ///
    /// [local-srv]: https://core.telegram.org/bots/api#using-a-local-bot-api-server
    #[must_use]
    pub fn server_url<S>(mut self, url: S) -> Self
    where
        S: Into<Url>,
    {
        self.server_url = Some(url.into());
        self
    }

    /// Specifies the bot token.
    ///
    /// See [Telegram Bot API: Authorizing your bot][token]
    ///
    /// [token]: https://core.telegram.org/bots/api#authorizing-your-bot
    ///
    /// This parameter is **required**.
    #[must_use]
    pub fn bot_token<T>(self, bot_token: T) -> TelegramSinkBuilder<String, ArgD>
    where
        T: Into<String>,
    {
        TelegramSinkBuilder {
            prop: self.prop,
            server_url: self.server_url,
            bot_token: bot_token.into(),
            recipient: self.recipient,
            silence: self.silence,
        }
    }

    /// Specifies the recipient of logs.
    ///
    /// This parameter is **required**.
    ///
    /// ## Examples
    ///
    /// ```
    /// use spdlog_telegram::{Recipient, TelegramSink};
    ///
    /// TelegramSink::builder()
    ///     // chat ID
    ///     .recipient(-1001234567890)
    ///     // or username
    ///     .recipient("@my_channel")
    ///     // or with thread ID
    ///     .recipient(
    ///         Recipient::builder()
    ///             .username("@my_chat")
    ///             .thread_id(114)
    ///             .build()
    ///     );
    /// ```
    #[must_use]
    pub fn recipient<R>(self, recipient: R) -> TelegramSinkBuilder<ArgT, Recipient>
    where
        R: Into<Recipient>,
    {
        TelegramSinkBuilder {
            prop: self.prop,
            server_url: self.server_url,
            bot_token: self.bot_token,
            recipient: recipient.into(),
            silence: self.silence,
        }
    }

    /// Specifies the silence level filter.
    ///
    /// Logs with level matching the filter will be sent with
    /// `disable_notification` set to `true`.
    ///
    /// This parameter is **optional**.
    #[must_use]
    pub fn silence(mut self, silent_if: LevelFilter) -> Self {
        self.silence = silent_if;
        self
    }

    // Prop
    //

    /// Specifies a log level filter.
    ///
    /// This parameter is **optional**.
    #[must_use]
    pub fn level_filter(self, level_filter: LevelFilter) -> Self {
        self.prop.set_level_filter(level_filter);
        self
    }

    /// Specifies a formatter.
    ///
    /// This parameter is **optional**.
    #[must_use]
    pub fn formatter<F>(self, formatter: F) -> Self
    where
        F: Formatter + 'static,
    {
        self.prop.set_formatter(formatter);
        self
    }

    /// Specifies an error handler.
    ///
    /// This parameter is **optional**.
    #[must_use]
    pub fn error_handler<F>(self, handler: F) -> Self
    where
        F: Into<ErrorHandler>,
    {
        self.prop.set_error_handler(handler);
        self
    }
}

impl<ArgR> TelegramSinkBuilder<(), ArgR> {
    #[doc(hidden)]
    #[deprecated(note = "\n\n\
        builder compile-time error:\n\
        - missing required field `bot_token`\n\n\
    ")]
    pub fn build(self, _: Infallible) {}
}

impl TelegramSinkBuilder<String, ()> {
    #[doc(hidden)]
    #[deprecated(note = "\n\n\
        builder compile-time error:\n\
        - missing required field `recipient`\n\n\
    ")]
    pub fn build(self, _: Infallible) {}
}

impl TelegramSinkBuilder<String, Recipient> {
    /// Builds a `TelegramSink`.
    pub fn build(self) -> Result<TelegramSink> {
        Ok(TelegramSink {
            prop: self.prop,
            silence: Atomic::new(self.silence),
            requester: Requester::new(
                self.server_url
                    .map_or_else(|| Url::parse("https://api.telegram.org"), Ok)
                    .map_err(Error::ParseUrl)?,
                &self.bot_token,
                self.recipient,
            )?,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use mockito::Matcher;
    use serde_json::json;

    use super::*;

    #[test]
    fn request() {
        let mut server = mockito::Server::new();

        let error_handler = |err| panic!("error handler triggered: {err}");
        let sink = Arc::new(
            TelegramSink::builder()
                .error_handler(error_handler)
                .server_url(Url::parse(&server.url()).unwrap())
                .bot_token("1234567890:AbCdEfGhiJkLmNoPq1R2s3T4u5V6w7X8y9z")
                .recipient(
                    Recipient::builder()
                        .chat_id(-1001234567890)
                        .thread_id(114)
                        .reply_to(514)
                        .build(),
                )
                .silence(LevelFilter::MoreVerboseEqual(Level::Info))
                .build()
                .unwrap(),
        );
        let logger = Logger::builder()
            .error_handler(error_handler)
            .sink(sink.clone())
            .build()
            .unwrap();

        let mut mocker = |level| {
            server
                .mock(
                    "POST",
                    "/bot1234567890:AbCdEfGhiJkLmNoPq1R2s3T4u5V6w7X8y9z/sendMessage",
                )
                .match_header("content-type", "application/json")
                .match_body(Matcher::PartialJson(json!({
                    "chat_id": -1001234567890_i64,
                    "disable_notification": sink.silence().test(level),
                    "link_preview_options": {
                        "is_disabled": true
                    },
                    "message_thread_id": 114,
                    "text": format!("#log #{} Hello Telegram! k=v", level.as_str()),
                    "reply_parameters": {
                        "message_id": 514,
                    }
                })))
                .with_header("content-type", "application/json")
                .with_body(json!({ "ok": true, "result": { /* omitted */ }}).to_string())
                .create()
        };

        let mock = mocker(Level::Info);
        info!(logger: logger, "Hello Telegram!", kv: { k = "v" });
        mock.assert();

        let mock = mocker(Level::Error);
        error!(logger: logger, "Hello Telegram!", kv: { k = "v" });
        mock.assert();
    }
}
