// TODO: Convert the implementation to use bounded channels.
use crate::data::{Ticket, TicketDraft};
use crate::store::{TicketId, TicketStore};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};

pub mod data;
pub mod store;

#[derive(Clone)]
pub struct TicketStoreClient {
    sender: SyncSender<Command>,
}

impl TicketStoreClient {
    pub fn insert(&self, draft: TicketDraft) -> Result<TicketId, OverloadedError> {
        let (sender, receiver) = sync_channel(1);
        let command = Command::Insert {
            draft,
            response_channel: sender
        };

        match self.sender.try_send(command) {
            Err(_) => return Err(OverloadedError),
            Ok(()) => ()
        }

        match receiver.recv() {
            Err(_) => panic!("You dun goofed"),
            Ok(id) => return Ok(id)
        };
    }

    pub fn get(&self, id: TicketId) -> Result<Option<Ticket>, OverloadedError> {
        let (sender, receiver): (SyncSender<Option<Ticket>>, Receiver<Option<Ticket>>) = sync_channel(1);
        let command = Command::Get { id, response_channel: sender };

        if let Result::Err(_) = self.sender.try_send(command){
            return Err(OverloadedError);     
        };

        Ok(receiver.recv().unwrap())
    }
}

pub fn launch(capacity: usize) -> TicketStoreClient {
    let (sender, receiver) = sync_channel(capacity);
    std::thread::spawn(move || server(receiver));
    TicketStoreClient {
        sender
    }
}

#[derive(Debug, thiserror::Error)]
#[error("The store is overloaded")]
pub struct OverloadedError;

enum Command {
    Insert {
        draft: TicketDraft,
        response_channel: SyncSender<TicketId>,
    },
    Get {
        id: TicketId,
        response_channel: SyncSender<Option<Ticket>>,
    },
}

pub fn server(receiver: Receiver<Command>) {
    let mut store = TicketStore::new();
    loop {
        match receiver.recv() {
            Ok(Command::Insert {
                draft,
                response_channel,
            }) => {
                let id = store.add_ticket(draft);
                let _ = response_channel.send(id);
            }
            Ok(Command::Get {
                id,
                response_channel,
            }) => {
                let ticket = store.get(id);
                let _ = response_channel.send(ticket.cloned());
            }
            Err(_) => {
                // There are no more senders, so we can safely break
                // and shut down the server.
                break;
            }
        }
    }
}
