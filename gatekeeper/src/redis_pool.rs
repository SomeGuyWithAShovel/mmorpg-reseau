use redis::{
    AsyncCommands,
    SetOptions,
};

use tokio::time::timeout;

use std::{
    net::{
        IpAddr, 
        Ipv4Addr,
    },
};

use serde::Deserialize;

#[allow(unused)]
use log::{info, warn, error};

use crate::handlers::{
    ServerInfo, ServerZone,
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

    if !add_raw_kv_ttl(&pool, key, val, ttl_sec).await
    {
        // error already printed in the function called
        return false;
    }

    let Some(result) = get_raw_v_from_k(&pool, key).await
    else
    {
        // error already printed in the function called
        return false;
    };

    if result != val
    {
        error!("The value associated with \"{}\" is \"{}\" and should be \"{}\"", key, result, val);
        return false;
    }
    
    return true;

}

// redis-cli SET key val (with ttl)
pub async fn add_raw_kv_ttl(pool: &RedisConnectionPool, key: &str, val: &str, ttl_sec: u64) -> bool
{
    // see comments of add_raw_kv_ttl_with_check()

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
            error!("Couldn't get a connection from the RedisConnectionPool.\n{}", _err);
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
                    error!("Couldn't set \"{} : {}\" into the Redis DB.\n{}", key, val, _err);
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

// TODO: Result<String, MyCustomError>, with enum MyCustomError { Timeout, RedisError, NoValue } or something similar
// redis-cli GET key
pub async fn get_raw_v_from_k(pool: &RedisConnectionPool, key: &str) -> Option<String>
{
    // see comments of add_raw_kv_ttl_with_check()
    
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
            error!("Couldn't get a connection from the RedisConnectionPool.\n{}", _err);
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
                    error!("Couldn't get value associated with \"{}\" from the Redis DB.\n{}", key, _err);
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

// redis-cli HGET key field
pub async fn get_str_from_key_and_field(pool: &RedisConnectionPool, key: &str, field: &str) -> Option<String>
{
    info!("get_str_from_key_and_field() : {} : {}", key, field);

    let Ok(get_conn_from_pool) = timeout(TIMEOUT_DURATION, 
        pool.get()
    ).await
    else
    {
        error!("Couldn't get a connection from the redis pool in time ({} seconds)", TIMEOUT_DURATION.as_secs_f32());
        return None;
    };

    if let Err(_err) = get_conn_from_pool
    { 
        error!("Couldn't get a connection from the RedisConnectionPool.\n{}", _err);
        return None;
    }

    let mut redis = get_conn_from_pool.unwrap();

    let Ok(redis_hget) = timeout(TIMEOUT_DURATION, redis.hget::<&str, &str, String>(key, field)).await
    else
    {
        error!("Couldn't get value of the key \"{}\" at the field \"{}\" from the Redis DB in time ({} seconds)", key, field, TIMEOUT_DURATION.as_secs_f32());
        return None;
    };

    if let Err(_err) = redis_hget
    {
        error!("Couldn't get value of the key \"{}\" at the field \"{}\" from the Redis DB.\n{}", key, field, _err);
        return None;
    }

    let result: String = redis_hget.unwrap();

    info!("Successfully got the value of the key \"{}\" at the field \"{}\" from the Redis DB, that is \"{}\"", key, field, result);

    return Some(result);
}

// redis-cli HSET key field value
pub async fn set_str_for_key_and_field(pool: &RedisConnectionPool, key: &str, field: &str, value: &str) -> bool
{
    info!("set_str_for_key_and_field() : \"{} : {}\" = {}", key, field, value);

    let Ok(get_conn_from_pool) = timeout(TIMEOUT_DURATION, 
        pool.get()
    ).await
    else
    {
        error!("Couldn't get a connection from the redis pool in time ({} seconds)", TIMEOUT_DURATION.as_secs_f32());
        return false;
    };

    if let Err(_err) = get_conn_from_pool
    { 
        error!("Couldn't get a connection from the RedisConnectionPool.\n{}", _err);
        return false;
    }

    let mut redis = get_conn_from_pool.unwrap();

    let Ok(redis_hset) = timeout(TIMEOUT_DURATION, redis.hset::<&str, &str, &str, isize>(key, field, value)).await
    else
    {
        error!("Couldn't set value of the key \"{}\" at the field \"{}\" to \"{}\" into the Redis DB in time ({} seconds)", key, field, value, TIMEOUT_DURATION.as_secs_f32());
        return false;
    };

    if let Err(_err) = redis_hset
    {
        error!("Couldn't set value of the key \"{}\" at the field \"{}\" to \"{}\" into the Redis DB.\n{}", key, field, value, _err);
        return false;
    }

    let result = redis_hset.unwrap(); // hset() returns the number of fields changed

    info!("Successfully set value of the key \"{}\" at the field \"{}\" to \"{}\" into the Redis DB (set {} values)", key, field, value, result);

    return true;
}

#[allow(unused)]
pub async fn test_redis_hget_hset(pool: &RedisConnectionPool) -> Result<String, String>
{
    if !set_str_for_key_and_field(&pool, "key", "field", "value").await
    {
        return Err("redis set didn't work".to_string());
    }

    let Some(result) = get_str_from_key_and_field(&pool, "key", "field").await
    else
    {
        return Err("redis get didn't work".to_string());
    };

    return Ok(format!("redis get worked and returned : {}", result));
}

// redis-cli HGETALL key
// returns a single String containing the (field, value) pairs in Json
pub async fn get_all_fields_from_key(pool: &RedisConnectionPool, key: &str) -> Option<String>
{
    info!("get_all_fields_from_key() : {}", key);

    let Ok(get_conn_from_pool) = timeout(TIMEOUT_DURATION, 
        pool.get()
    ).await
    else
    {
        error!("Couldn't get a connection from the redis pool in time ({} seconds)", TIMEOUT_DURATION.as_secs_f32());
        return None;
    };

    if let Err(_err) = get_conn_from_pool
    { 
        error!("Couldn't get a connection from the RedisConnectionPool.\n{}", _err);
        return None;
    }

    let mut redis = get_conn_from_pool.unwrap();

    let Ok(redis_hgetall) = timeout(TIMEOUT_DURATION, redis.hgetall::<&str, Vec<(String, String)>>(key)).await
    else
    {
        error!("Couldn't get all fields from the key \"{}\" in time ({} seconds)", key, TIMEOUT_DURATION.as_secs_f32());
        return None;
    };

    if let Err(_err) = redis_hgetall
    {
        error!("Couldn't get all fields from the key \"{}\".\n{}", key, _err);
        return None;
    }
    let result_vec = redis_hgetall.unwrap();

    // hgetall returns [("field1", "value1"), ("field2", "value2", ...)]

    // we need : """{ "field1" : "value1", "field2" : value2 }"""
    // surrounded by {}, values not surrounded by "" if they are numbers
    let result_string = format!(
        "{{{}}}", // { is the escape character for {, so I need 2 to actually have a '{' in the string, and a third to do like a %s in C

        result_vec.iter()
        .map(
            |(k, v)|
            {
                if v.parse::<f64>().is_ok() || v.parse::<i64>().is_ok() || v.parse::<u64>().is_ok() // should treat all numbers
                {
                    format!("\"{}\" : {}", k, v) // v shouldn't be surrounded by ""
                }
                else
                {
                    format!("\"{}\" : \"{}\"", k, v)
                }
                // there is probably issues with arrays or idk what else value json supports
            }
        )
        .collect::<Vec<String>>() // apply the map and gets a Vec<String> (from a Vec<(String, String)>)
        .join(", ") // transform the Vec<String> into a single String with a separator between each one
    );


    info!("Successfully got all fields from the key \"{}\" : {}", key, result_string);

    return Some(result_string);
}

// -------------------------------------------------------------------------------------------------------------------

// scan_match, while loop that returns when the element satisfy the conditions
pub async fn get_first_key_matching_pattern_and_cond(pool: &RedisConnectionPool, pattern: &str, cond: fn(&str) -> bool) -> Option<String>
{
    info!("get_first_key_matching_pattern_and_cond() : pattern = {}", pattern);

    let Ok(get_conn_from_pool) = timeout(TIMEOUT_DURATION, 
        pool.get()
    ).await
    else
    {
        error!("Couldn't get a connection from the redis pool in time ({} seconds)", TIMEOUT_DURATION.as_secs_f32());
        return None;
    };

    if let Err(_err) = get_conn_from_pool
    { 
        error!("Couldn't get a connection from the RedisConnectionPool.\n{}", _err);
        return None;
    }

    let mut redis = get_conn_from_pool.unwrap();


    // https://docs.rs/redis/latest/redis/struct.AsyncIter.html

    let scan_result = redis.scan_match::<&str, String>(pattern).await;
    if let Err(_err) = scan_result
    {
        error!("Couldn't get an async iterator to scan keys matching \"{}\".\n{}", pattern, _err);
        return None;
    }
    let mut iter = scan_result.unwrap();

    while let Some(iter_result) = iter.next_item().await
    {
        if let Err(_err) = iter_result
        {
            error!("Error while iterating through a scan matching the pattern \"{}\"", pattern);
            return None;
        }
        let element = iter_result.unwrap();

        if cond(&element) == true
        {
            info!("Found an element that matches \"{}\" and satisfies the condition : {}", pattern, element);
            return Some(element);
        }
    }
    
    return None;
}

// -------------------------------------------------------------------------------------------------------------------

// because what we get from redis isn't what we want to return in our REST API

#[derive(Deserialize)]
struct ServerInfoInRedis
{
    ip: Ipv4Addr,
    port: u16,
    zone: String,
    #[allow(dead_code)]
    player_count: isize,
    #[allow(dead_code)]
    status: String
}

impl ServerZone
{
    fn from_string(str: &str) -> Option<ServerZone>
    {
        match str
        {
            "zone_A" => { Some(ServerZone::zone_A) }
            "zone_B" => { Some(ServerZone::zone_B) }
            "zone_C" => { Some(ServerZone::zone_C) }
            "zone_D" => { Some(ServerZone::zone_D) }
            "zone_E" => { Some(ServerZone::zone_E) }
            _ => { None }
        }
    }
}

impl ServerInfoInRedis
{
    fn into_server_info(&self) -> Option<ServerInfo>
    {
        let Some(zone) = ServerZone::from_string(&self.zone)
        else { return None; };
        return Some( ServerInfo {
            ip: self.ip,
            port: self.port,
            zone: zone,
        });
    }
}

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

    let Some(server_key) = get_first_key_matching_pattern_and_cond(pool, "server:*", |_str|{ true }).await
    else
    {
        return None;
    };

    let Some(server_string) = get_all_fields_from_key(pool, &server_key).await
    else
    {
        return None;   
    };

    match serde_json::from_str::<ServerInfoInRedis>(&server_string)
    {
        Err(_err) =>
        {
            error!("Couldn't deserialize into a ServerInfoInRedis.\n{}", _err);
            return None;
        }
        Ok(redis_server_info) =>
        {
            info!("Serialized into ServerInfoInRedis successfully");

            let Some(server_info) = redis_server_info.into_server_info()
            else
            {
                error!("Conversion from ServerInfoInRedis into ServerInfo failed");
                return None;
            };

            info!("Successfully translated ServerInfoInRedis into ServerInfo");
            return Some(server_info);
        }
    }
}