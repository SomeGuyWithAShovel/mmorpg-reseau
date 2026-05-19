use axum::{
    Router, 
    routing::{
        get,
        post,
    },
    http::StatusCode, 
    response::{
        IntoResponse, 
        Response
    }, 
    Json, 
    extract::{
        State,
        ConnectInfo,
    }
};

use serde::{ 
    Serialize,
    Deserialize,
};

use std::net::{
        Ipv4Addr, 
        SocketAddr,
    };

use uuid::Uuid;

#[allow(unused)]
use log::{info, warn, error};

use crate::{
    AppStateDyn,
    redis_pool::{
        find_server,
    }
};

// -------------------------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------------------------

pub async fn router() -> Router<AppStateDyn>
{
    return Router::new()
        .route("/health", get(get_status))
        .route("/login", post(try_login))
    ;
}

// -------------------------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------------------------

pub async fn handler_404(
) -> String
{
    info!("handler_404");
    return "404 string".to_string();
}

// -------------------------------------------------------------------------------------------------------------------

pub async fn root_handler(
    State(state): State<AppStateDyn>
) -> (StatusCode, String)
{
    info!("root_handler()");

    let response: String = format!("hello from {}", state.arc_mutex.lock().await.get_name());
    return (StatusCode::OK, response);
}

// -------------------------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------------------------

#[derive(Serialize)]
pub struct ServiceStatus
{
    pub status: String,
}

// -------------------------------------------------------------------------------------------------------------------

async fn get_status(
    #[allow(unused)] State(state): State<AppStateDyn>,
) -> (StatusCode, Json<ServiceStatus>)
{
    info!("get_status()");

    let status = ServiceStatus { status: String::from("ok"), };
    return (StatusCode::OK, Json(status));
}

// -------------------------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------------------------

// POST /login Json INPUT
#[derive(Debug, Deserialize)]
struct LoginRequest
{
    username: String,
    password: String,
}

// -------------------------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------------------------

// POST /login Json OUTPUT

#[derive(Serialize)]
struct LoginSuccess
{
    player_id: Uuid,
    server: ServerInfo,
}

// -------------------------------------------------------------------------------------------------------------------

// TODO : move to a shared lib

#[derive(Clone, Copy, Serialize, Deserialize)]
#[allow(non_camel_case_types, unused)]
pub enum ServerZone
{
    zone_A,
    zone_B,
    zone_C,
    zone_D,
    zone_E,
    // ...
}

#[derive(Serialize, Deserialize)]
pub struct ServerInfo
{
    pub ip: Ipv4Addr,
    pub port: u16,
    pub zone: ServerZone,
}

// -------------------------------------------------------------------------------------------------------------------

#[derive(Serialize)]
struct LoginUnauthorized
{
    error: String,
}
impl Default for LoginUnauthorized
{
    fn default() -> Self 
    {
        return Self { 
            error: String::from("Authentification failed"), 
        };
    }
}

#[derive(Serialize)]
struct LoginUnavailable
{
    error: String,
}
impl Default for LoginUnavailable
{
    fn default() -> Self 
    {
        return Self { 
            error: String::from("No server available"), 
        };
    }
}


enum LoginResponse
{
    Success(LoginSuccess),
    Unauthorized(LoginUnauthorized),
    Unavailable(LoginUnavailable),
}

impl IntoResponse for LoginResponse
{
    fn into_response(self) -> Response
    {   
        match self
        {   
            LoginResponse::Success(success) =>
            {   
                return (StatusCode::OK, Json(success)).into_response();
            }
            LoginResponse::Unauthorized(unauthorized) =>
            {
                return (StatusCode::UNAUTHORIZED, Json(unauthorized)).into_response();
            }
            LoginResponse::Unavailable(unavailable) =>
            {   
                return (StatusCode::SERVICE_UNAVAILABLE, Json(unavailable)).into_response();
            }
        }
    }
}

// -------------------------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------------------------

async fn try_login(
    State(state): State<AppStateDyn>,
    ConnectInfo(sock_addr): ConnectInfo<SocketAddr>,
    Json(login_request): Json<LoginRequest>
) -> Response
{
    info!("try_login : {:?} (from {})", login_request, sock_addr);
    
    let Some(player_uuid) = login_validate(&login_request).await
    else
    {
        return LoginResponse::Unauthorized(LoginUnauthorized::default()).into_response();
    };

    let mut app_state = state.arc_mutex.lock().await;


    let Some(redis_connection_pool) = app_state.get_redis_connection_pool()
    else
    {
        return LoginResponse::Unavailable(LoginUnavailable::default()).into_response();
    };
    let Some(server_info) = find_server(redis_connection_pool, sock_addr.ip()).await
    else
    {
        return LoginResponse::Unavailable(LoginUnavailable::default()).into_response();
    };

    let success = LoginSuccess {
        player_id: player_uuid,
        server: server_info
    };

    return LoginResponse::Success(success).into_response();
}

async fn login_validate(login: &LoginRequest) -> Option<Uuid>
{
    let _ = login.username.clone(); // removing the unused warning for login.username, 
    // but here, where it should be used (to query the user DB), not globally with a macro when declaring the field of the LoginRequest struct

    // password hashing ?
    // query some DB to find UUID ?
    if login.password == "1234"
    {
        return Some(Uuid::new_v4());
    }
    else
    {
        return None;    
    }
}

// -------------------------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------------------------




