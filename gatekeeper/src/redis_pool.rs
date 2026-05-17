use redis::{
    AsyncCommands, 
    SetOptions,
};

use tokio::time::timeout;

use std::{
    net::{
        IpAddr,
    },
};

#[allow(unused)]
use log::{info, warn, error};

use crate::handlers::{
    ServerInfo,
};

pub const TIMEOUT_DURATION : std::time::Duration = std::time::Duration::from_millis(2000);

// -------------------------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------------------------

pub type RedisConnectionPool = bb8::Pool<redis::Client>;

pub async fn create_redis_pool(redis_string: &str) -> Option<RedisConnectionPool>
{
    // redis_string = "redis://localhost"
    info!("create_redis_pool()");

    info!("client");
    let Ok(client) = redis::Client::open(redis_string) // TODO:  should probably use TLS
    else 
    {
        error!("create_redis_pool() : {} couldn't be opened", redis_string);
        return None;
    };

    info!("pool");
    // see https://docs.rs/bb8/latest/bb8/ for why we use a connection pool
    let Ok(pool) = bb8::Pool::builder().build(client).await
    else 
    {
        error!("create_redis_pool() : pool creation failed");
        return None;
    };

    return Some(pool);
}

// -------------------------------------------------------------------------------------------------------------------

// sets a value with a key, then gets the value associated with key
pub async fn add_raw_kv_ttl_with_check(pool: &RedisConnectionPool, key: &str, val: &str, ttl_sec: u64) -> bool
{
    info!("add_raw_kv_ttl_with_check() : \"{} : {}\"", key, val);

    // https://docs.rs/redis/latest/redis/struct.SetOptions.html
    // https://redis.io/docs/latest/commands/set/
    let set_options = SetOptions::default()
        .with_expiration( if ttl_sec == 0_u64 { redis::SetExpiry::KEEPTTL } else { redis::SetExpiry::EX(ttl_sec) } )
    ;

    // I used let Ok(var) else when calling timeout(duration, foo()).await
    // because we don't care about the Err() result of timeout()

    // However, when it succeeds, we get a Result<T, E>, and we care about both T and E,
    // so we can't use let ... else {} or if let ... {} with early returns, because it would discard either T or E

    // So I used match blocks, but it shifts the useful code to the right (nested indentation)


    // if   : async fn foo() -> Result<T,E>, 
    // then : timeout(t, foo()).await -> Result< Result<T, E>, Elapsed>
    //
    // in our case, we don't really care about Elapsed, so we can use let Ok() else {}.
    // but we care about T and E

    let Ok(get_conn_from_pool) = timeout(TIMEOUT_DURATION, 
        pool.get()
    ).await
    else
    {
        error!("Couldn't get a connection from the redis pool in time ({} seconds)", TIMEOUT_DURATION.as_secs_f32());
        return false;
    };

    match get_conn_from_pool
    {
        Err(_err) =>
        {
            error!("Couldn't get a connection from the RedisConnectionPool : {}", _err);
            return false;
        }
        Ok(mut redis_conn) => 
        {
            info!("Redis connection retrieved successfully");

            let Ok(redis_set) = timeout(TIMEOUT_DURATION, 
                redis_conn.set_options::<&str, &str, ()>(key, val, set_options)
            ).await
            else
            {
                error!("Couldn't set \"{} : {}\" into the Redis DB in time ({} seconds)", key, val, TIMEOUT_DURATION.as_secs_f32());
                return false;
            };

            match redis_set
            {
                Err(_err) => 
                {
                    error!("Couldn't set \"{} : {}\" into the Redis DB : {}", key, val, _err);
                    return false;
                }
                Ok(_) =>
                {
                    info!("Successfully set \"{} : {}\" into the Redis DB", key, val);

                    let Ok(redis_get) = timeout(TIMEOUT_DURATION,
                        redis_conn.get::<&str, String>(key)
                    ).await
                    else 
                    {
                        error!("Couldn't get value associated with \"{}\" from the Redis DB in time ({} seconds)", key, TIMEOUT_DURATION.as_secs_f32());
                        return false;
                    };
        
                    match redis_get
                    {
                        Err(_err) => 
                        {
                            error!("Couldn't get value associated with \"{}\" from the Redis DB : {}", key, _err);
                            return false;
                        }
                        Ok(result) =>
                        {
                            info!("Successfully got the value associated with \"{}\" from the Redis DB, that is \"{}\"", key, result);
                            if result != val
                            {
                                error!("The value associated with \"{}\" is \"{}\" and should be \"{}\"", key, result, val);
                                return false;
                            }
                            return true;
                        }
                    }
                }
            }
        }
    }
}

// see comments of add_raw_kv_ttl_with_check()
pub async fn add_raw_kv_ttl(pool: &RedisConnectionPool, key: &str, val: &str, ttl_sec: u64) -> bool
{
    info!("add_raw_kv_ttl() : \"{} : {}\"", key, val);

    let set_options = SetOptions::default()
        .with_expiration( if ttl_sec == 0_u64 { redis::SetExpiry::KEEPTTL } else { redis::SetExpiry::EX(ttl_sec) } )
    ;

    let Ok(get_conn_from_pool) = timeout(TIMEOUT_DURATION, 
        pool.get()
    ).await
    else
    {
        error!("Couldn't get a connection from the redis pool in time ({} seconds)", TIMEOUT_DURATION.as_secs_f32());
        return false;
    };

    match get_conn_from_pool
    {
        Err(_err) =>
        {
            error!("Couldn't get a connection from the RedisConnectionPool : {}", _err);
            return false;
        }
        Ok(mut redis_conn) => 
        {
            info!("Redis connection retrieved successfully");

            let Ok(redis_set) = timeout(TIMEOUT_DURATION, 
                redis_conn.set_options::<&str, &str, ()>(key, val, set_options)
            ).await
            else
            {
                error!("Couldn't set \"{} : {}\" into the Redis DB in time ({} seconds)", key, val, TIMEOUT_DURATION.as_secs_f32());
                return false;
            };

            match redis_set
            {
                Err(_err) => 
                {
                    error!("Couldn't set \"{} : {}\" into the Redis DB : {}", key, val, _err);
                    return false;
                }
                Ok(_) =>
                {
                    info!("Successfully set \"{} : {}\" into the Redis DB", key, val);
                    return true;
                }
            }
        }
    }
}

// see comments of add_raw_kv_ttl_with_check()
// TODO: Result<String, MyCustomError>, with enum MyCustomError { Timeout, RedisError, NoValue } or something similar
pub async fn get_v_from_k(pool: &RedisConnectionPool, key: &str) -> Option<String>
{
    info!("get_v_from_k() : \"{}\"", key);

    let Ok(get_conn_from_pool) = timeout(TIMEOUT_DURATION, 
        pool.get()
    ).await
    else
    {
        error!("Couldn't get a connection from the redis pool in time ({} seconds)", TIMEOUT_DURATION.as_secs_f32());
        return None;
    };

    match get_conn_from_pool
    {
        Err(_err) =>
        {
            error!("Couldn't get a connection from the RedisConnectionPool : {}", _err);
            return None;
        }
        Ok(mut redis_conn) => 
        {
            info!("Redis connection retrieved successfully");

            let Ok(redis_get) = timeout(TIMEOUT_DURATION,
                redis_conn.get::<&str, String>(key)
            ).await
            else 
            {
                error!("Couldn't get value associated with \"{}\" from the Redis DB in time ({} seconds)", key, TIMEOUT_DURATION.as_secs_f32());
                return None;
            };
        
            match redis_get
            {
                Err(_err) => 
                {
                    error!("Couldn't get value associated with \"{}\" from the Redis DB : {}", key, _err);
                    return None;
                }
                Ok(result) =>
                {
                    info!("Successfully got the value associated with \"{}\" from the Redis DB, that is \"{}\"", key, result);
                    return Some(result);
                }
            }
        }
    }
}

// -------------------------------------------------------------------------------------------------------------------

pub async fn find_server(
    pool: &mut RedisConnectionPool,
    #[allow(unused)] player_location_ip: IpAddr
) -> Option<ServerInfo>
{
    // TODO : 
    // - find the ServerZone corresponding to player_location_ip
    // - query Redis to get a server in that zone, that isn't full yet has the most players (we don't want 99 servers with 1 players each)

    // for now, a server is hardcoded into Redis :
    // redis-cli SET "server" "{\"ip\" : \"111.222.111.222\", \"port\" : 54321, \"zone\" : \"zone_A\"}"
    // the \ are necessary for serde_json to correctly deserialize, but aren't needed to add a string that looks like it's correct into Redis

    let Some(server_string) = get_v_from_k(pool, "server").await
    else
    {
        return None;
    };

    match serde_json::from_str::<ServerInfo>(&server_string)
    {
        Err(_err) =>
        {
            error!("Couldn't deserialize into a ServerInfo : {}", _err);
            return None;
        }
        Ok(server_info) =>
        {
            info!("Serialized into ServerInfo successfully");
            return Some(server_info);
        }
    }
}