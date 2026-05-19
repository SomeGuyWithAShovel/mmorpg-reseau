use tokio::{net::UdpSocket, sync::mpsc, sync::mpsc::Sender, time::{self, Duration}, task::JoinSet};
use std::io;
use std::sync::{Arc, Mutex};
use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use bytes::*;
use shared::*;
use std::process::{Command, Child};

extern crate r2d2;
extern crate redis;
use redis::TypedCommands;

const KEY_TTL : f32 = 15.0;
const DEFAULT_HEARTBEAT_PORT : u16 = 47347;
const DEFAULT_SERVER_CMD_PATH : &str = "../../dedicated_server/target/debug/mmo_dedicated_server";
const FIRST_SERVER_PORT : u16 = 8080;

#[tokio::main]
async fn main() -> io::Result<()> {

    // Récupération des variables d'environnement
    let hot_servers_min = std::env::var("HOT_SERVERS_MIN")
        .ok()
        .map(|s| s.parse::<usize>().ok())
        .flatten()
        .unwrap_or(0);
    let port = std::env::var("ORCH_PORT")
        .ok()
        .map(|s| s.parse::<u16>().ok())
        .flatten()
        .unwrap_or(DEFAULT_HEARTBEAT_PORT);
    let server_cmd_path = std::env::var("SERVER_CMD_PATH").unwrap_or(DEFAULT_SERVER_CMD_PATH.to_string());
    let mut next_server_port = FIRST_SERVER_PORT;
    
    // Création du socket de heartbeat
    let sock = UdpSocket::bind(SocketAddr::new(IpAddr::from(Ipv4Addr::new(0, 0, 0, 0)), port)).await?;
    let r = Arc::new(sock);
    let (tx, mut rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(1_000);

    // Création de la connection Redis
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let pool = r2d2::Pool::builder().build(client).unwrap();

    // Réception heartbeat
    tokio::spawn(async move {
        let buf = [0; 1024];
        loop {
            heartbeat_listener(&r, buf, &tx).await;
        }
    });

    // Mise à jour de Redis selon le heartbeat
    let mut heartbeat_conn = pool.get().expect("Connection à redis échoué");
    tokio::spawn(async move {
        while let Some((bytes, _)) = rx.recv().await {
            if let Some(heartbeat) = Heartbeat::from_bytes(Bytes::from(bytes)) {
                println!("Heartbeat : {:?}", heartbeat);
                if let Err(_) = update_redis_ttl(heartbeat, &mut heartbeat_conn).await {
                    println!("Update redis ttl erreur");
                }
            }
            else {
                println!("Heartbeat invalide lors du parse");
            }
        }
    });


    // Gestion du spawn de serveurs    
    let mut server_count_interval = time::interval(Duration::from_secs(2));
    let mut children : Vec<Child> = Vec::new();
    let should_quit = Arc::new(Mutex::new(false));

    // Si on reçoit un Ctrl+C, l'orchestrateur et tous les DS créés sont libérés
    let ctrl_c_recieved = Arc::clone(&should_quit);
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        let mut should_quit_value = ctrl_c_recieved.lock().unwrap();
        *should_quit_value = true;
    });

    let mut count_server_conn = pool.get().expect("Connection à redis échoué");
    while !*should_quit.lock().unwrap() {
        let count = count_available_servers(&mut count_server_conn).await;
        if count < hot_servers_min {
            let mut set = JoinSet::new();
            
            for _ in count..hot_servers_min {
                let server_cmd_path_view = server_cmd_path.clone();
                set.spawn(async move {
                    spawn_server(&server_cmd_path_view, next_server_port).await
                });
                next_server_port += 1;
            }
            let new_children = set.join_all().await;
            children.extend(new_children);
        }
        server_count_interval.tick().await;
    }

    // Arrêt de tous les processus créés en cas d'arrêt manuel
    for child in &mut children {
        child.kill()?;
    }
    Ok(())
}

async fn heartbeat_listener(r : &Arc<UdpSocket>, mut buf : [u8; 1024], tx : &Sender<(Vec<u8>, SocketAddr)>) {
    if let Ok((len, addr)) = r.recv_from(&mut buf).await {
        println!("{:?} bytes received from {:?}, {:?}", len, addr, &buf.to_vec()[18..63]);
        tx.send((buf[18..len].to_vec(), addr)).await.unwrap();
    }
    else {
        println!("Help");
    }
}

async fn spawn_server(server_cmd_path : &String, port : u16) -> Child {
    Command::new(server_cmd_path)
        .env("DS_PORT", port.to_string())
        .spawn()
        .expect("La création du serveur a échoué")
}

async fn count_available_servers(conn : &mut r2d2::PooledConnection<redis::Client>) -> usize {

    let mut count = 0;
    
    if let Ok(servers) = conn.keys("server:*") {
        for server in servers {
            if let Ok(server_info) = conn.hget(server, "status") && let Some(availability) = server_info {
                if availability == "available" {
                    count += 1;
                }
            }
        }
    }
    return count;
}

async fn update_redis_ttl(heartbeat : Heartbeat, conn : &mut r2d2::PooledConnection<redis::Client>) -> redis::RedisResult<()> {
    let key = format!("server:{}", heartbeat.id);
    redis::cmd("HSET").arg(key.as_str())
        .arg("ip").arg(heartbeat.addr.ip().to_string().as_str())
        .arg("port").arg(heartbeat.addr.port())
        .arg("zone").arg(heartbeat.zone)
        .arg("player_count").arg(heartbeat.player_count)
        .arg("status").arg(if heartbeat.is_full {"full"} else {"available"})
        .exec(conn)?;

    redis::cmd("EXPIRE").arg(key.as_str()).arg(KEY_TTL).exec(conn)?;
    println!("Updated redis !");
    Ok(())
}
