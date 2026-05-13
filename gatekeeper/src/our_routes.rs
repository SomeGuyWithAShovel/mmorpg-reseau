use axum::{
    Router, 
    routing::get,
    response::Html, 
};

#[allow(unused)]
use log::{info, warn, error};



pub async fn router() -> Router {
    Router::new()
        .route("/", get(handler))
}



pub async fn on_404() -> String
{
    info!("on_404()");
    return "404 string".to_string();
}



async fn handler() -> Html<&'static str>
{
    info!("route_handler");
    return Html("<h1>route_handler</h1>");
}
