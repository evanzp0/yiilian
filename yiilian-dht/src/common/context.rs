use std::{panic::RefUnwindSafe, sync::{Mutex, RwLock}};

use yiilian_core::common::shutdown::ShutdownReceiver;

use crate::{
    net::Client, peer::PeerManager, routing_table::RoutingTable, transaction::TransactionManager
};

use super::{setting::Settings, state::State};

pub struct Context {
    settings: Settings,
    state: RwLock<State>,
    routing_table: Mutex<RoutingTable>,
    peer_manager: Mutex<PeerManager>,
    transaction_manager: TransactionManager,
    client: Client,
    shutdown_rx: ShutdownReceiver,
}

impl Context {
    pub fn new(
        settings: Settings,
        state: RwLock<State>,
        routing_table: Mutex<RoutingTable>,
        peer_manager: Mutex<PeerManager>,
        transaction_manager: TransactionManager,
        client: Client,
        shutdown_rx: ShutdownReceiver,
    ) -> Self {
        Context {
            settings,
            state,
            routing_table,
            peer_manager,
            transaction_manager,
            client,
            shutdown_rx,
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

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn shutdown_rx(&self) -> ShutdownReceiver {
        self.shutdown_rx.clone()
    }
}

impl RefUnwindSafe for Context {}