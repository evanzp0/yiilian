use std::{collections::HashMap, panic::RefUnwindSafe, sync::{Mutex, RwLock}};

use once_cell::sync::OnceCell;

use crate::{
    except_option, net::Client, peer::PeerManager, routing_table::RoutingTable, transaction::TransactionManager
};

use super::{setting::Settings, state::State};

pub static mut DHT_CONTEXT: OnceCell<HashMap<u16, Context>> = OnceCell::new();

pub struct Context {
    settings: Settings,
    state: RwLock<State>,
    routing_table: Mutex<RoutingTable>,
    peer_manager: Mutex<PeerManager>,
    transaction_manager: TransactionManager,
    client: Client,
}

impl Context {
    pub fn new(
        settings: Settings,
        state: RwLock<State>,
        routing_table: Mutex<RoutingTable>,
        peer_manager: Mutex<PeerManager>,
        transaction_manager: TransactionManager,
        client: Client,
    ) -> Self {
        Context {
            settings,
            state,
            routing_table,
            peer_manager,
            transaction_manager,
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

    pub fn client(&self) -> &Client {
        &self.client
    }
}

impl RefUnwindSafe for Context {}

pub fn dht_ctx(ctx_index: u16) -> &'static Context {
    let ctx = unsafe { except_option!(DHT_CONTEXT.get(), "DHT_CONTEXT get() is None") };
    let ctx = except_option!(ctx.get(&ctx_index), "Item in DHT_CONTEXT Map is not set");

    ctx
}

pub fn dht_ctx_insert(ctx_index: u16, context: Context) {
    unsafe { 
        DHT_CONTEXT.get_or_init(|| {
            HashMap::new()
        });

        let map = except_option!(DHT_CONTEXT.get_mut(), "DHT_CONTEXT get_mut() is None");
        map.insert(ctx_index, context);
    };
}

pub fn dht_ctx_settings(ctx_index: u16) -> &'static Settings {
    dht_ctx(ctx_index).settings()
}

pub fn dht_ctx_state(ctx_index: u16) -> &'static RwLock<State> {
    dht_ctx(ctx_index).state()
}

pub fn dht_ctx_routing_tbl(ctx_index: u16) -> &'static Mutex<RoutingTable> {
    dht_ctx(ctx_index).routing_table()
}

pub fn dht_ctx_peer_mgr(ctx_index: u16) -> &'static Mutex<PeerManager> {
    dht_ctx(ctx_index).peer_manager()
}

pub fn dht_ctx_trans_mgr(ctx_index: u16) -> &'static TransactionManager {
    dht_ctx(ctx_index).transaction_manager()
}

pub fn dht_ctx_client(ctx_index: u16) -> &'static Client {
    dht_ctx(ctx_index).client()
}
