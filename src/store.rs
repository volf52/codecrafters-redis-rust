use std::collections::HashMap;

use tokio::{
    stream::StreamExt,
    sync::{broadcast, mpsc},
};

use crate::expiring_cache::ExpiringValue;

#[derive(Debug)]
pub enum StoreCommand {
    Get {
        key: String,
    },
    Set {
        key: String,
        value: String,
        expiry: Option<std::time::Duration>,
    },
}

impl StoreCommand {
    pub fn get_value(key: String) -> Self {
        Self::Get { key }
    }

    pub fn set_value(key: String, value: String, expiry: Option<std::time::Duration>) -> Self {
        Self::Set { key, value, expiry }
    }
}

// could use option as well
#[derive(Debug, Clone, PartialEq)]
pub enum StoreResponse {
    Value(String),
    Nil,
    Ok,
}

#[derive(Debug)]
pub struct KVStore(HashMap<String, ExpiringValue<String>>);

impl Default for KVStore {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl KVStore {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub async fn run_loop(
        &mut self,
        mut command_receiver: mpsc::Receiver<StoreCommand>,
        response_sender: broadcast::Sender<StoreResponse>,
    ) {
        while let Some(cmd) = command_receiver.next().await {
            println!("Received command: {:?}", cmd);

            let resp = match cmd {
                StoreCommand::Get { key } => {
                    let val = self.0.get(&key);

                    if let Some(v) = val {
                        if v.has_expired() {
                            println!("Removing expired value for key <{}>", key);
                            self.0.remove(&key);
                            StoreResponse::Nil
                        } else {
                            StoreResponse::Value(v.value.clone())
                        }
                    } else {
                        StoreResponse::Nil
                    }
                }
                StoreCommand::Set { key, value, expiry } => {
                    let val = ExpiringValue::new(value, expiry);

                    self.0.insert(key, val);

                    StoreResponse::Ok
                }
            };

            let _res = response_sender.send(resp);
        }
    }
}
