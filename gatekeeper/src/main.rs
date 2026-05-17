// see https://github.com/tokio-rs/axum/blob/main/examples
// (readme, graceful-shutdown, dependency-injection, ...)

use tokio::{
    signal,
};

use axum::{
    Router,
    routing::get,
};

#[allow(unused)]
use log::{info, warn, error};

use std::{
    net::{
        Ipv4Addr,
        SocketAddrV4,
    }, 
    sync::Arc
};

mod handlers;

// -------------------------------------------------------------------------------------------------------------------
// TODO : from env variables

pub const LISTEN_IP: [u8; 4] = [127, 0, 0, 1];
pub const LISTEN_PORT:  u16  = 3000;

pub const fn get_socket_addr_v4(ip: [u8; 4], port: u16) -> SocketAddrV4
{
    return SocketAddrV4::new(Ipv4Addr::from_octets(ip), port);
}

pub const LISTEN_SOCK_ADDR_V4: SocketAddrV4 = get_socket_addr_v4(LISTEN_IP, LISTEN_PORT);

pub const SERVICE_NAME: &str = "Gatekeeper";

// -------------------------------------------------------------------------------------------------------------------

// -------------------------------------------------------------------------------------------------------------------

// Things required for Dependency Injection (using trait objects with dyn, instead of generics)

#[derive(Clone)]
pub struct AppStateData
{
    pub name: String,
    pub sock_addr: SocketAddrV4,
}

pub trait AppStateTrait: Send + Sync
{
    fn get_name(&self) -> &String;
    fn get_sock_addr(&self) -> &SocketAddrV4;
}

impl AppStateTrait for AppStateData
{
    fn get_name(&self) -> &String { return &self.name; }
    fn get_sock_addr(&self) -> &SocketAddrV4 { return &self.sock_addr; }
}

#[derive(Clone)]
pub struct AppStateDyn
{
    pub data: Arc<dyn AppStateTrait>,
}

// -------------------------------------------------------------------------------------------------------------------

#[tokio::main]
async fn main()
{
    // TODO : from env variables
    let service_state = AppStateData {
        name: String::from(SERVICE_NAME),
        sock_addr: LISTEN_SOCK_ADDR_V4,
    };
    
    println!("Starting {} (on {})", service_state.name, service_state.sock_addr);

    // allow info!() logging without needing to set any environment variables
    env_logger::Builder::new().filter_level(log::LevelFilter::Info).parse_default_env().init();
    
    let Ok(listener) = tokio::net::TcpListener::bind(service_state.sock_addr).await else {
        panic!("Couldn't bind TCP socket on address {}", service_state.sock_addr);
    };

    info!("Listening on address {}", service_state.sock_addr);
    
    // non mut, so we configure it in a single chain (app = r.foo().bar().foo2()... ; // app is now immutable)
    let app = Router::new() 
    
        .route("/", get(handlers::root_handler))

        // we can "split" our routes into submodules, and add their routers here, with their routes nested in the path provided
        // .nest("/some_path", some_Router.await)

        // we can't nest routers at the root path, so we need to use merge
        .merge(handlers::router().await) 
        
        // dependency injection, AFTER adding the different routers with handlers that uses it, not before
        .with_state(AppStateDyn { data: Arc::new(service_state.clone()) } )  

        // handler for errors
        .fallback(handlers::handler_404)
    ;

    axum::serve(listener, app)

        // allows to CTRL-C the server while not killing it abruptly
        .with_graceful_shutdown(setup_shutdown()) 

        .await.unwrap()
    ;
    
    println!("Stopping Server ({})", service_state.sock_addr);
    info!("end of main() reached");
}

async fn setup_shutdown()
{
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            // info!("on_shutdown() : CTRL-C");
            on_shutdown().await
        },
        _ = terminate => {
            // info!("on_shutdown() : terminate");
            on_shutdown().await
        },
    }
}

async fn on_shutdown()
{
    info!("on_shutdown");
}