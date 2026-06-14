/* 
 *  Écrit à l'aide des exemples issus de la documentation tokio et de redis-rs :
 *  - https://docs.rs/tokio/latest/tokio/net/struct.UdpSocket.html#example-splitting-with-arc
 *  - https://docs.rs/tokio/latest/tokio/signal/fn.ctrl_c.html#examples
 *  - https://docs.rs/tokio/latest/tokio/sync/struct.Notify.html
 */

use tokio::sync::{mpsc::{self, Sender, Receiver}, Notify};
use tokio::{net::UdpSocket, time::{self, Duration}, task::JoinSet};
use std::io;
use std::sync::{Arc, Mutex};
use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use bytes::*;
use shared::*;
use std::process::{Command, Child};
use log::{info, warn, error};
extern crate r2d2;
extern crate redis;
use redis::TypedCommands;

const KEY_TTL : i64 = 15;
const DEFAULT_SERVER_CMD_PATH : &str = "../dedicated_server/target/debug/mmo_dedicated_server";
const DEFAULT_SERVER_COUNT_INTERVAL : u64 = 3;
const FIRST_SERVER_PORT : u16 = 8080;

#[tokio::main]
async fn main() -> io::Result<()> {
    env_logger::init();
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
        .unwrap_or(DEFAULT_ORCH_PORT);
    let server_count_interval_duration = std::env::var("SERVER_COUNT_INTERVAL")
        .ok()
        .map(|s| s.parse::<u64>().ok())
        .flatten()
        .unwrap_or(DEFAULT_SERVER_COUNT_INTERVAL);
    let server_cmd_path = std::env::var("SERVER_CMD_PATH")
        .unwrap_or(DEFAULT_SERVER_CMD_PATH.to_string());
    let mut next_server_port = FIRST_SERVER_PORT;
    
    info!("Nombre de serveur minimal voulu = {}", hot_servers_min);
    info!("Port orchestrateur d'écoute des heartbeats = {}", port);
    info!("Intervalle entre comptage des serveurs = {}", server_count_interval_duration);
    info!("Chemin de fichier vers la commande de création de serveur = {}", server_cmd_path);
    
    // Création du socket de heartbeat
    let sock = UdpSocket::bind(SocketAddr::new(IpAddr::from(Ipv4Addr::new(0, 0, 0, 0)), port)).await?;
    let r = Arc::new(sock);
    let (tx, mut rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(1_000);

    // Création de la connection Redis
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let pool = r2d2::Pool::builder().build(client).unwrap();

    // Réception heartbeat
    tokio::spawn(async move { heartbeat_listener(&r, &tx).await; });

    // Mise à jour de Redis selon le heartbeat
    let mut heartbeat_conn = pool.get().expect("Connection à redis échouée");
    tokio::spawn(async move { heartbeat_consumer (&mut heartbeat_conn, &mut rx).await; });

    let next_iteration = Arc::new(Notify::new());

    // Gestion du spawn de serveurs    
    let mut children : Vec<Child> = Vec::new();
    let should_quit = Arc::new(Mutex::new(false));

    let mut count_server_conn = pool.get().expect("Connection à redis échouée");
    let mut server_count_interval = time::interval(Duration::from_secs(server_count_interval_duration));

    // Demande l'itération suivante de la boucle principale toutes les n secondes
    // On le fait en dehors de la boucle principale pour assurer que ctrl+c puisse
    // lui aussi modifier next_iteration quand il en a besoin
    //
    // Et, accessoirement, on s'assure que l'on passe bien "server_count_interval.duration"
    // entre chaque itération (sinon, on attendait cette durée plus la durée de l'exécution
    // de la boucle principale
    let regular_next_iteration = next_iteration.clone();

    // Si on reçoit un Ctrl+C, l'orchestrateur et tous les DS créés sont libérés
    let ctrl_c_recieved = Arc::clone(&should_quit);
    let ctrl_c_end_iteration = Arc::clone(&next_iteration);
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        let mut should_quit_value = ctrl_c_recieved.lock().unwrap();
        *should_quit_value = true;
        // On veut sortir de la boucle principale sans attendre la prochaine itération
        ctrl_c_end_iteration.notify_one();
        info!("Arrêt des subprocess...");
    });
    
    server_count_interval.tick().await;
    tokio::spawn(async move {
        loop {
            server_count_interval.tick().await;
            regular_next_iteration.notify_one();
        }
    });
    
    while !*should_quit.lock().unwrap() {
        let count = count_available_servers(&mut count_server_conn).await;
        if count < hot_servers_min {
            let mut set = JoinSet::new();
            
            for _ in count..hot_servers_min {
                let server_cmd_path_view = server_cmd_path.clone();
                set.spawn(async move {
                    spawn_server(&server_cmd_path_view, next_server_port, port).await
                });
                info!("Spawn d'un serveur avec port {}", next_server_port);
                next_server_port += 1;
            }
            let new_children = set.join_all().await;
            children.extend(new_children);
        }
        next_iteration.notified().await;
    }

    // Arrêt de tous les processus créés en cas d'arrêt manuel
    for child in &mut children {
        child.kill()?;
    }
    Ok(())
}

async fn heartbeat_listener(r : &Arc<UdpSocket>, tx : &Sender<(Vec<u8>, SocketAddr)>) {
    let mut buf = [0; 1024];
    while let Ok((len, addr)) = r.recv_from(&mut buf).await {
        // 18 octets pour un paquet envoyé par game_sockets::protocols::UdpProtocol
        info!("{:?} octers reçu de {:?}, {:?}", len, addr, &buf.to_vec()[18..len]);
        tx.send((buf[18..len].to_vec(), addr)).await.unwrap();
    }
}

async fn heartbeat_consumer(conn : &mut r2d2::PooledConnection<redis::Client>, rx : &mut Receiver<(Vec<u8>, SocketAddr)>) {
    while let Some((bytes, _)) = rx.recv().await {
        if let Some(heartbeat) = Heartbeat::from_bytes(Bytes::from(bytes)) {
            info!("Heartbeat : {:?}", heartbeat);
            if let Err(err) = update_redis_ttl(heartbeat, conn).await {
                error!("Erreur mise à jour du TTL Redis : {:?}", err);
            }
        }
        else {
            warn!("Heartbeat invalide lors du parse");
        }
    }
}

async fn spawn_server(server_cmd_path : &String, server_port : u16, own_port : u16) -> Child {
    Command::new(server_cmd_path)
        .env("DS_PORT", server_port.to_string())
        .env("DS_ZONE", "zone_A")
        .env("DS_MAX_PLAYERS", "16")
        .env("ORCH_ADDRESS", "127.0.0.1")
        .env("ORCH_PORT", own_port.to_string())
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
    info!("{} serveurs trouvés", count);
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
    info!("Mise à jour de redis avec succès !");
    Ok(())
}
