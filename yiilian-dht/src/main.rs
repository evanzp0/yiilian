use std::{collections::HashSet, convert::Infallible, net::SocketAddr, sync::{Arc, Mutex, RwLock}};

use tokio::net::UdpSocket;
use yiilian_core::{
    common::{error::Error, shutdown::create_shutdown, util::random_bytes}, net::block_list::{BlockAddr, BlockList}, service::ServiceBuilder
};
use yiilian_dht::{
    common::{context::Context, id::Id, ip::IPV4Consensus, setting::SettingsBuilder, state::State}, event::EventManager, net::{
        service::{make_service_fn, DummyService}, Client, Server
    }, peer::PeerManager, routing_table::RoutingTable, transaction::TransactionManager
};

#[tokio::main]
async fn main() -> Result<(), Error> {
    setup_log();

    let (mut _shutdown_tx, shutdown_rx) = create_shutdown();

    let local_addr: SocketAddr = "0.0.0.0:6578".parse().unwrap();
    let local_id = Id::from_ip(&local_addr.ip());

    let settings = SettingsBuilder::new()
        // .routers(&config.dht.routers)
        .build();

    let node_file_prefix = Some("dht".to_owned());
    let state = RwLock::new(build_state(
        local_addr, 
        local_id, 
        settings.token_secret_size, 
        &node_file_prefix
    )?);

    let block_list = None;
    let routing_table = Mutex::new(build_routing_table(
        local_id,
        settings.block_list_max_size, 
        settings.bucket_size,
        block_list,
    )?);

    let peer_manager = Mutex::new(PeerManager::new(
        settings.max_resources,
        settings.max_peers_per_resource,
    ));

    let transaction_manager = TransactionManager::new(local_addr, shutdown_rx.clone());

    let event_manager = EventManager::new(shutdown_rx.clone());

    let socket = Arc::new(build_socket(local_addr)?);
    let client = Client::new(socket.clone());

    let ctx = Context::new(
        settings,
        state,
        routing_table,
        peer_manager,
        transaction_manager,
        event_manager,
        client
    );
    let ctx = Arc::new(ctx);
    
    let make_service = make_service_fn(|ctx: Arc<Context>| async move {
        let dummy = DummyService::new(ctx.clone());
        let svc = ServiceBuilder::new().service(dummy);

        Ok::<_, Infallible>(svc)
    });

    let server = Server::new(socket.clone(), make_service, ctx);
    server.run_loop().await?;

    Ok(())
}

fn build_state(
    local_addr: SocketAddr,
    local_id: Id,
    token_secret_size: usize,
    node_file_prefix: &Option<String>,
) -> Result<State, Error> {
    let port = local_addr.port();
    let token_secret = random_bytes(token_secret_size);
    let node_file_prefix = if let Some(prefix) = node_file_prefix {
        prefix.to_owned() + "_"
    } else {
        "".to_owned()
    };

    let nodes_file = home::home_dir()
        .map_or(
            Err(Error::new_path(
                None,
                Some(format!("<user home> not found")),
            )),
            |v| Ok(v),
        )?
        .join(format!(".yiilian/dht/{}{}.txt", node_file_prefix, port));

    Ok(State::new(
        local_id,
        IPV4Consensus::new(2, 10),
        token_secret,
        local_addr,
        nodes_file,
    ))
}

fn build_routing_table(
    local_id: Id,
    block_list_max_size: i32, 
    bucket_size: usize,
    block_list: Option<HashSet<BlockAddr>>
) -> Result<RoutingTable, Error> {
    let block_list = BlockList::new(block_list_max_size, block_list);
    let routing_table = RoutingTable::new(
        bucket_size,
        block_list,
        local_id,
    );

    Ok(routing_table)
}

fn build_socket(socket_addr: SocketAddr) -> Result<UdpSocket, Error> {
    let std_sock = std::net::UdpSocket::bind(socket_addr)
        .map_err(|e| Error::new_bind(Some(Box::new(e))))?;
    std_sock
        .set_nonblocking(true)
        .map_err(|e| Error::new_bind(Some(Box::new(e))))?;

    let socket =
        UdpSocket::from_std(std_sock).map_err(|e| Error::new_bind(Some(Box::new(e))))?;

    Ok(socket)
}

fn setup_log() {
    dotenv::dotenv().ok();
    env_logger::init();
}