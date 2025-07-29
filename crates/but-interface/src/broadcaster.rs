use std::collections::HashMap;

use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FrontendEvent {
    name: String,
    payload: serde_json::Value,
}

pub struct Broadcaster {
    senders: HashMap<uuid::Uuid, tokio::sync::mpsc::UnboundedSender<FrontendEvent>>,
}

impl Broadcaster {
    pub fn send(&self, event: FrontendEvent) {
        for sender in self.senders.values() {
            let _ = sender.send(event.clone());
        }
    }
}
