use std::{collections::HashSet, net::SocketAddr};

use yiilian_core::{
    common::{error::Error, shutdown::ShutdownReceiver},
    net::block_list::BlockAddr,
    service::{Identity, Layer, ServiceBuilder, Stack},
};

use crate::{
    common::{Settings, SettingsBuilder},
    data::body::KrpcBody,
    service::{KrpcService, RouterService},
};

use super::{Dht, DhtMode};

pub struct DhtBuilder<L, S> {
    local_addr: SocketAddr,
    service_builder: ServiceBuilder<L>,
    router_service: S,
    settings: Option<Settings>,
    block_list: Option<HashSet<BlockAddr>>,
    shutdown_rx: ShutdownReceiver,
    workers: Option<usize>,
    mode: DhtMode,
}

impl DhtBuilder<Identity, RouterService> {
    pub fn new(local_addr: SocketAddr, shutdown_rx: ShutdownReceiver, workers: Option<usize>) -> Self {
        let router_service = RouterService::new(local_addr);
        Self {
            local_addr,
            service_builder: ServiceBuilder::new(),
            router_service,
            settings: None,
            block_list: None,
            shutdown_rx,
            workers,
            mode: DhtMode::Normal,
        }
    }

    pub fn settings(mut self, settings: Option<Settings>) -> Self {
        self.settings = settings;
        self
    }

    pub fn block_list(mut self, block_list: Option<HashSet<BlockAddr>>) -> Self {
        self.block_list = block_list;
        self
    }

    pub fn mode(mut self, mode: DhtMode) -> Self {
        self.mode = mode;
        self
    }
}

impl<L, S> DhtBuilder<L, S> {
    pub fn layer<T>(self, layer: T) -> DhtBuilder<Stack<T, L>, S> {
        let service_builder = ServiceBuilder {
            layer: Stack::new(layer, self.service_builder.layer),
        };

        DhtBuilder {
            local_addr: self.local_addr,
            service_builder,
            router_service: self.router_service,
            settings: self.settings,
            block_list: self.block_list,
            shutdown_rx: self.shutdown_rx,
            workers: self.workers,
            mode: self.mode,
        }
    }

    pub fn build(self) -> Result<Dht<L::Service>, Error>
    where
        L: Layer<S>,
        L::Service:
            KrpcService<KrpcBody, ResBody = KrpcBody, Error = Error> + Clone + Send + 'static,
    {
        let service = self.service_builder.service(self.router_service);
        let dht = Dht::init(
            self.local_addr,
            service,
            self.settings.unwrap_or(SettingsBuilder::new().build()),
            self.block_list,
            self.shutdown_rx,
            self.workers,
            self.mode,
        );

        dht
    }
}
