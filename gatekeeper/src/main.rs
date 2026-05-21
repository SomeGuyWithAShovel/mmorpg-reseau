// see https://github.com/tokio-rs/axum/blob/main/examples
// (readme, graceful-shutdown, dependency-injection, ...)

use axum::{
    Router,
    routing::get,
};

use std::{
    net::{
        Ipv4Addr,
        SocketAddrV4,
        SocketAddr
    }, 
    sync::{
        Arc,
    }
};

use tokio::{
    signal,
};

#[allow(unused)]
use log::{info, warn, error};

// use std::sync::Mutex; // they are blocking
use tokio::sync::Mutex;// they are async / non-blocking

use crate::redis_pool::{
    TIMEOUT_DURATION,
    RedisConnectionPool, 
};

mod handlers;
mod redis_pool;

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

pub const REDIS_CLIENT_STRING: &str = "redis://localhost";

// -------------------------------------------------------------------------------------------------------------------

// -------------------------------------------------------------------------------------------------------------------

// Things required for Dependency Injection (using trait objects with dyn, instead of generics)

#[derive(Clone)]
pub struct AppStateData
{
    pub name: String,
    pub sock_addr: SocketAddrV4,
    pub redis_connection_pool: Option<RedisConnectionPool>,
}

pub trait AppStateTrait: Send + Sync
{
    fn get_name(&self) -> &String;
    fn get_sock_addr(&self) -> &SocketAddrV4;
    fn get_redis_connection_pool(&mut self) -> Option<&mut RedisConnectionPool>;
}

impl AppStateTrait for AppStateData
{
    fn get_name(&self) -> &String { return &self.name; }
    fn get_sock_addr(&self) -> &SocketAddrV4 { return &self.sock_addr; }
    fn get_redis_connection_pool(&mut self) -> Option<&mut RedisConnectionPool> { return self.redis_connection_pool.as_mut(); }
}

#[derive(Clone)]
// "Invoking clone on Arc produces a new Arc instance, which points to the same allocation on the heap as the source Arc, while increasing a reference count"
// and Mutex allows to provide the ability to lock and get a mutable reference, to modify the state.
// It could be a blocking std::sync::Mutex, but since in the state we also store a RedisConnectionPool that uses async methods, we need it to be an async tokio::sync::Mutex
pub struct AppStateDyn
{
    pub arc_mutex: Arc<Mutex<dyn AppStateTrait>>,
}

// -------------------------------------------------------------------------------------------------------------------

#[tokio::main]
async fn main()
{
    // TODO : from env variables
    let service_state_arcmutex = Arc::new(Mutex::new(
        
        AppStateData {
            name: String::from(SERVICE_NAME),
            sock_addr: LISTEN_SOCK_ADDR_V4,
            redis_connection_pool: None,
        }
    ));
    
    // I need a scope for accessing the service_state inside the arc_mutex,
    // I need the data inside that service_state to create the TCP socketn
    // but I need the TcpListener to outlive the scope of the mutex.

    let listener = {

        let mut service_state = service_state_arcmutex.lock().await;
    
        println!("Starting {} (on {})", service_state.name, service_state.sock_addr);

        // allow info!() logging without needing to set any environment variables
        env_logger::Builder::new().filter_level(log::LevelFilter::Info).parse_default_env().init();

        service_state.redis_connection_pool = redis_pool::create_redis_pool(REDIS_CLIENT_STRING).await;
        
        {
            let Some(redis_connection_pool) = &service_state.redis_connection_pool
            else 
            {
                panic!("Couldn't create redis client \"{}\"", REDIS_CLIENT_STRING);
            };

            if redis_pool::add_raw_kv_ttl_with_check(&redis_connection_pool, "ping", "pong", TIMEOUT_DURATION.as_secs() * 3).await == false
            {
                panic!("Couldn't ping redis client \"{}\" before {} seconds timeout", REDIS_CLIENT_STRING, redis_pool::TIMEOUT_DURATION.as_secs_f32());
            }
        }

        let Ok(listener) = tokio::net::TcpListener::bind(service_state.sock_addr).await 
        else 
        {
            panic!("Couldn't bind TCP socket on address {}", service_state.sock_addr);
        };

        info!("Listening on address {}", service_state.sock_addr);

        listener
    };
    
    // non mut, so we configure it in a single chain (app = r.foo().bar().foo2()... ; // app is now immutable)
    let app = Router::new() 
    
        .route("/", get(handlers::root_handler))

        // we can "split" our routes into submodules, and add their routers here, with their routes nested in the path provided
        // .nest("/some_path", some_Router.await)

        // we can't nest routers at the root path, so we need to use merge
        .merge(handlers::router().await) 
        
        // dependency injection, AFTER adding the different routers with handlers that uses it, not before
        .with_state(AppStateDyn { arc_mutex: service_state_arcmutex.clone() } )  

        // handler for errors
        .fallback(handlers::handler_404)
    ;

    axum::serve(
            listener, 
            app

                // allows handlers to use a parameter of type ConnectInfo<std::net::SocketAddr>
                // there might be issues with proxies
                .into_make_service_with_connect_info::<SocketAddr>()
        )

        // allows to CTRL-C the server while not killing it abruptly
        .with_graceful_shutdown(setup_shutdown()) 

        .await.unwrap()
    ;

    // no scope : we keep the lock until we return
    let service_state = service_state_arcmutex.lock().await;

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