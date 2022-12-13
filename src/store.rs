use std::collections::HashMap;

use tokio::{
    stream::StreamExt,
    sync::{broadcast, mpsc},
};

#[derive(Debug)]
pub enum StoreCommand {
    Get { key: String },
    Set { key: String, value: String },
}

impl StoreCommand {
    pub fn get_value(key: String) -> Self {
        Self::Get { key }
    }

    pub fn set_value(key: String, value: String) -> Self {
        Self::Set { key, value }
    }
}

// could use option as well
#[derive(Debug, Clone)]
pub enum StoreResponse {
    Value(String),
    Nil,
    Ok,
}

#[derive(Debug, Default)]
pub struct KVStore(HashMap<String, String>);

impl KVStore {
    pub fn new() -> Self {
        KVStore::default()
    }

    pub async fn run_loop(
        &mut self,
        mut command_receiver: mpsc::Receiver<StoreCommand>,
        response_sender: broadcast::Sender<StoreResponse>,
    ) {
        while let Some(cmd) = command_receiver.next().await {
            println!("Received command: {:?}", cmd);

            let resp = match cmd {
                StoreCommand::Get { key } => self
                    .0
                    .get(&key)
                    .map(|v| StoreResponse::Value(v.clone()))
                    .unwrap_or(StoreResponse::Nil),
                StoreCommand::Set { key, value } => {
                    let entry = self.0.entry(key).or_insert_with(|| value.clone());
                    *entry = value;

                    StoreResponse::Ok
                }
            };

            let _res = response_sender.send(resp);
        }
    }
}
