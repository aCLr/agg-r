use super::parsers::parse_update;
use crate::result::Result;
use crate::updates::SourceData;
use std::sync::Arc;
use std::thread::JoinHandle;
use tg_collector::tg_client::{TgClient, TgUpdate};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::task::spawn;

/// Handler interacts with tdlib using `tg_collector` crate.
/// It initializes updates listener and pass all updates from `tg_collector` to specified sender

#[derive(Clone)]
pub struct Handler {
    sender: Arc<Mutex<mpsc::Sender<Result<SourceData>>>>,
    tg: Arc<RwLock<TgClient>>,
    orig_sender: mpsc::Sender<TgUpdate>,
    orig_receiver: Arc<Mutex<mpsc::Receiver<TgUpdate>>>,
}

impl Handler {
    /// Creates new Handler with specified
    pub fn new(
        sender: Arc<Mutex<mpsc::Sender<Result<SourceData>>>>,
        tg: Arc<RwLock<TgClient>>,
    ) -> Self {
        // TODO: configure channel size
        let (orig_sender, orig_receiver) = mpsc::channel::<TgUpdate>(2000);
        Self {
            sender,
            tg,
            orig_sender,
            orig_receiver: Arc::new(Mutex::new(orig_receiver)),
        }
    }

    pub async fn run(&mut self) -> JoinHandle<()> {
        let mut guard = self.tg.write().await;
        guard.start_listen_updates(self.orig_sender.clone());
        // TODO handle join
        let join_handle = guard.start();
        let recv = self.orig_receiver.clone();
        let sender = self.sender.clone();
        spawn(async move {
            loop {
                let update = recv.lock().await.recv().await;
                match &update {
                    None => return,
                    Some(update) => {
                        let parsed_update = match parse_update(update).await {
                            Ok(Some(update)) => Ok(SourceData::Telegram(update)),
                            Err(e) => Err(e),

                            Ok(None) => continue,
                        };
                        let mut local_sender = sender.lock().await;

                        if let Err(err) = local_sender.send(parsed_update).await {
                            warn!("{}", err)
                        }
                    }
                }
            }
        });
        join_handle
    }
}
