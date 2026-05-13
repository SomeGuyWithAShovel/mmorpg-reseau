use tokio::{
    signal,
};

use axum::{
    routing::get,
    Router
};

#[allow(unused)]
use log::{info, warn, error};

mod common; use common::*;
mod our_routes;


#[tokio::main]
async fn main()
{
    println!("Starting Server ({})", LISTEN_SOCK_ADDR_V4);

    // allow info!() logging without needing to set any environment variables
    env_logger::Builder::new().filter_level(log::LevelFilter::Info).parse_default_env().init();
    


    let app = Router::new() // non mut, so we configure it in a single chain (app = r.foo().bar().foo2()... ,)
        .route(
            "/", 
            get( // get, post, ...
                || async {  info!("handler||"); "Hello, World!" } // handler : a callable. Could be a function, or like here, a closure
            )
        )
        .nest("/routes", our_routes::router().await) // we can "split" our routes into submodules, and add their routers here, with their routes nested in the path provided
        .fallback(our_routes::on_404) // handler for errors
    ;

    let Ok(listener) = tokio::net::TcpListener::bind(LISTEN_SOCK_ADDR_V4).await else {
        panic!("Couldn't bind TCP socket on address {}", LISTEN_SOCK_ADDR_V4);
    };

    info!("Listening on address {}", LISTEN_SOCK_ADDR_V4);

    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(setup_shutdown()) // allows to CTRL-C the server while not killing it abruptly
        .await.unwrap();

    info!("end of main() reached");
}

async fn setup_shutdown()
{
    // from : https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs

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
            info!("on_shutdown() : CTRL-C");
            on_shutdown().await
        },
        _ = terminate => {
            info!("on_shutdown() : terminate");
            on_shutdown().await
        },
    }
}

async fn on_shutdown()
{
    println!("Stopping Server ({})", LISTEN_SOCK_ADDR_V4);
}