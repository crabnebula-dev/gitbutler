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
    pub fn new() -> Self {
        Self {
            senders: HashMap::new(),
        }
    }

    pub fn send(&self, event: FrontendEvent) {
        for sender in self.senders.values() {
            let _ = sender.send(event.clone());
        }
    }

    pub fn register_sender(
        &mut self,
        id: &uuid::Uuid,
        sender: tokio::sync::mpsc::UnboundedSender<FrontendEvent>,
    ) {
        self.senders.insert(*id, sender);
    }

    pub fn deregister_sender(&mut self, id: &uuid::Uuid) {
        self.senders.remove(id);
    }
}
