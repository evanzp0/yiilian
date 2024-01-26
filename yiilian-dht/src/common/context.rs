use std::{net::SocketAddr, panic::RefUnwindSafe, sync::{Mutex, RwLock}};

use crate::{
    net::Client, peer::PeerManager, routing_table::RoutingTable, transaction::TransactionManager
};

use super::{setting::Settings, state::State};

pub struct Context {
    local_addr: SocketAddr,
    settings: Settings,
    state: RwLock<State>,
    routing_table: Mutex<RoutingTable>,
    peer_manager: Mutex<PeerManager>,
    transaction_manager: TransactionManager,
    client: Client,
}

impl Context {
    pub fn new(
        local_addr: SocketAddr,
        settings: Settings,
        state: RwLock<State>,
        routing_table: Mutex<RoutingTable>,
        peer_manager: Mutex<PeerManager>,
        transaction_manager: TransactionManager,
        client: Client,
    ) -> Self {
        Context {
            local_addr,
            settings,
            state,
            routing_table,
            peer_manager,
            transaction_manager,
            client,
        }
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
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

    pub fn client(&self) -> &Client {
        &self.client
    }
}

impl RefUnwindSafe for Context {}