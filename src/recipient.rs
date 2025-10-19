use std::{borrow::Cow, convert::Infallible};

use serde_json as json;

#[derive(Debug, PartialEq, Eq)]
enum TargetChatInner {
    Id(i64),
    Username(String),
}

// Hacky: `TargetChat` is private, but we have to set it to `pub` instead of
// `pub(crate)` because it appears as generic in `RecipientBuilder`.
pub(crate) mod __private {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    pub struct TargetChat(TargetChatInner);

    impl TargetChat {
        pub(crate) fn id(chat_id: i64) -> Self {
            Self(TargetChatInner::Id(chat_id))
        }

        pub(crate) fn username(username: String) -> Self {
            Self(TargetChatInner::Username(username))
        }

        pub(crate) fn into_json(self) -> json::Value {
            match self.0 {
                TargetChatInner::Id(id) => json::Value::Number(id.into()),
                TargetChatInner::Username(username) => json::Value::String(username),
            }
        }
    }
}
use __private::TargetChat;

/// Represents a Telegram chat recipient.
///
/// Not just a chat ID or username, it can also be represented with a message
/// thread ID or reply.
#[derive(Debug, PartialEq, Eq)]
pub struct Recipient {
    pub(crate) target: TargetChat,
    pub(crate) thread_id: Option<u64>,
    pub(crate) reply_to: Option<(u64, Option<TargetChat>)>,
}

impl Recipient {
    /// Gets a builder for `Recipient`.
    pub fn builder() -> RecipientBuilder<()> {
        RecipientBuilder {
            target: (),
            thread_id: None,
            reply_to: None,
        }
    }

    /// Constructs a `Recipient` from a chat ID.
    ///
    /// This is equivalent to `Recipient::builder().chat_id(chat_id).build()`.
    pub fn chat_id(chat_id: i64) -> Self {
        Self::builder().chat_id(chat_id).build()
    }

    /// Constructs a `Recipient` from a username.
    ///
    /// This is equivalent to `Recipient::builder().username(username).build()`.
    pub fn username<S>(username: S) -> Self
    where
        S: Into<String>,
    {
        Self::builder().username(username).build()
    }
}

impl From<i64> for Recipient {
    fn from(chat_id: i64) -> Self {
        Self::chat_id(chat_id)
    }
}

macro_rules! impl_from_str_for_recipient {
    ( $($str_ty:ty),+ ) => {
        $(impl From<$str_ty> for Recipient {
            fn from(username: $str_ty) -> Self {
                Self::username(username)
            }
        })+
    };
}
impl_from_str_for_recipient!(&str, &mut str, Box<str>, Cow<'_, str>, String, &String);

pub struct RecipientBuilder<ArgC> {
    target: ArgC,
    thread_id: Option<u64>,
    reply_to: Option<(u64, Option<TargetChat>)>,
}

impl<ArgC> RecipientBuilder<ArgC> {
    pub fn chat_id(self, chat_id: i64) -> RecipientBuilder<TargetChat> {
        RecipientBuilder {
            target: TargetChat::id(chat_id),
            thread_id: self.thread_id,
            reply_to: self.reply_to,
        }
    }

    pub fn username<S>(self, username: S) -> RecipientBuilder<TargetChat>
    where
        S: Into<String>,
    {
        RecipientBuilder {
            target: TargetChat::username(username.into()),
            thread_id: self.thread_id,
            reply_to: self.reply_to,
        }
    }

    pub fn thread_id(mut self, thread_id: u64) -> Self {
        self.thread_id = Some(thread_id);
        self
    }

    pub fn reply_to(mut self, message_id: u64) -> Self {
        self.reply_to = Some((message_id, None));
        self
    }

    // It's not a very good name, and considering there's almost no use case for it,
    // I chose not to make it public for now.
    #[allow(dead_code)]
    fn reply_to_diff_chat_id(mut self, message_id: u64, chat_id: i64) -> Self {
        self.reply_to = Some((message_id, Some(TargetChat::id(chat_id))));
        self
    }

    // Same as above.
    #[allow(dead_code)]
    fn reply_to_diff_username<S>(mut self, message_id: u64, chat_username: S) -> Self
    where
        S: Into<String>,
    {
        self.reply_to = Some((message_id, Some(TargetChat::username(chat_username.into()))));
        self
    }
}

impl RecipientBuilder<()> {
    #[doc(hidden)]
    #[deprecated(note = "\n\n\
        builder compile-time error:\n\
        - missing required field `chat_id` or `username`\n\n\
    ")]
    pub fn build(self, _: Infallible) {}
}

impl RecipientBuilder<TargetChat> {
    pub fn build(self) -> Recipient {
        Recipient {
            target: self.target,
            thread_id: self.thread_id,
            reply_to: self.reply_to,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_into() {
        fn echo(r: impl Into<Recipient>) -> Recipient {
            r.into()
        }
        assert_eq!(echo(-1001234567890), Recipient::chat_id(-1001234567890));
        assert_eq!(echo("@username"), Recipient::username("@username"));
    }
}
