use axum::{
    Router, 
    debug_handler, 
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
    extract::State
};

use serde::{ 
    Serialize,
    Deserialize,
};

use std::{
    net::{
        Ipv4Addr,
    },
};

use uuid::Uuid;

#[allow(unused)]
use log::{info, warn, error};

use crate::{
    AppStateDyn,
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

#[debug_handler]
pub async fn handler_404(
) -> String
{
    info!("handler_404");
    return "404 string".to_string();
}

// -------------------------------------------------------------------------------------------------------------------

#[debug_handler]
pub async fn root_handler(
    State(state): State<AppStateDyn>
) -> (StatusCode, String)
{
    info!("root_handler()");

    let response: String = format!("hello from {}", state.data.get_name());
    return (StatusCode::OK, response);
}

// -------------------------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------------------------

#[derive(Serialize)]
struct ServiceStatus
{
    status: String,
}

// -------------------------------------------------------------------------------------------------------------------

#[debug_handler]
async fn get_status(
    #[allow(unused)] State(state): State<AppStateDyn>
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

#[derive(Serialize)]
#[allow(non_camel_case_types, unused)]
enum ServerZones
{
    zone_A,
    zone_B,
    zone_C,
    zone_D,
    zone_E,
    // ...
}

#[derive(Serialize)]
struct ServerInfo
{
    ip: Ipv4Addr,
    port: u16,
    zone: ServerZones,
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

#[debug_handler]
async fn try_login(
    Json(login_request): Json<LoginRequest>
) -> Response
{
    info!("try_login : {:?}", login_request);
    
    let Some(player_uuid) = login_validate(&login_request).await
    else
    {
        return LoginResponse::Unauthorized(LoginUnauthorized::default()).into_response();
    };

    let Some(server_info) = login_find_server(&login_request).await
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

async fn login_find_server(#[allow(unused)] login: &LoginRequest) -> Option<ServerInfo>
{
    if true
    {
        return Some(
            ServerInfo { 
                ip: Ipv4Addr::from_octets([1,2,3,4]), 
                port: 32769_u16, 
                zone: ServerZones::zone_A,
            }
        );
    }
    else
    {
        return None;
    }
}
// -------------------------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------------------------




