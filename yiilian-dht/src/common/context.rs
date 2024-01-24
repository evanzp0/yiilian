use std::sync::{Mutex, RwLock};

use crate::{
    event::EventManager, net::Client, peer::PeerManager, routing_table::RoutingTable, transaction::TransactionManager
};

use super::{setting::Settings, state::State};

pub struct Context {
    settings: Settings,
    state: RwLock<State>,
    routing_table: Mutex<RoutingTable>,
    peer_manager: Mutex<PeerManager>,
    transaction_manager: TransactionManager,
    event_manager: EventManager,
    client: Client,
}

impl Context {
    pub fn new(
        settings: Settings,
        state: RwLock<State>,
        routing_table: Mutex<RoutingTable>,
        peer_manager: Mutex<PeerManager>,
        transaction_manager: TransactionManager,
        event_manager: EventManager,
        client: Client,
    ) -> Self {
        Context {
            settings,
            state,
            routing_table,
            peer_manager,
            transaction_manager,
            event_manager,
            client,
        }
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn state(&self) -> &RwLock<State> {
        &self.state
    }

    pub fn routing_table(&self) -> &Mutex<RoutingTable> {
        &self.routing_table
    }

    pub fn peer_manager(&self) -> &Mutex<PeerManager> {
        &self.peer_manager
    }

    pub fn transaction_manager(&self) -> &TransactionManager {
        &self.transaction_manager
    }

    pub fn event_manager(&self) -> &EventManager {
        &self.event_manager
    }

    pub fn client(&self) -> &Client {
        &self.client
    }
}
