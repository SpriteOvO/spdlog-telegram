use reqwest::header::CONTENT_TYPE;
use serde_json::{self as json, json};
use url::Url;

use crate::{Error, Recipient, Result};

pub(crate) struct Requester {
    client: reqwest::blocking::Client,
    endpoint: Url,
    payload: json::Value,
}

impl Requester {
    pub(crate) fn new(server_url: Url, bot_token: &str, recipient: Recipient) -> Result<Self> {
        let mut payload = json!({
            "chat_id": recipient.target.into_json(),
            "message_thread_id": recipient.thread_id,
            "text": null,
            "link_preview_options": {
                "is_disabled": true,
            },
            "disable_notification": null,
        });

        // Telegram server requires the field `reply_parameters` must be an object or
        // not present, but a JSON `null` will be rejected.
        if let Some((message_id, target)) = recipient.reply_to {
            let payload = payload.as_object_mut().unwrap();
            payload.insert(
                "reply_parameters".into(),
                json!({
                    "message_id": message_id,
                    "chat_id": target.map(|t| t.into_json()),
                }),
            );
        }

        Ok(Self {
            client: reqwest::blocking::Client::new(),
            endpoint: server_url
                .join(&format!("/bot{}/sendMessage", bot_token))
                .map_err(Error::ParseUrl)?,
            payload,
        })
    }

    pub(crate) fn send_log(&self, text: String, disable_notification: bool) -> Result<()> {
        let mut payload = self.payload.as_object().unwrap().clone();
        payload["text"] = json::Value::String(text);
        payload["disable_notification"] = json::Value::Bool(disable_notification);
        let payload = json::Value::Object(payload);

        let response = self
            .client
            .post(self.endpoint.as_str())
            .header(CONTENT_TYPE, "application/json")
            .body(payload.to_string())
            .send()
            .map_err(|err| Error::SendRequest(err.into()))?;

        let status_unsuccess = !response.status().is_success();
        let (ok, description) = response
            .text()
            .ok()
            .and_then(|resp| json::from_str::<json::Value>(&resp).ok())
            .and_then(|resp| {
                resp.as_object().map(|resp| {
                    (
                        resp.get("ok").and_then(|j| j.as_bool()).unwrap_or(false),
                        resp.get("description")
                            .and_then(|j| j.as_str().map(str::to_string)),
                    )
                })
            })
            .unwrap_or((false, None));

        if status_unsuccess || !ok {
            Err(Error::TelegramApi(description))
        } else {
            Ok(())
        }
    }
}
