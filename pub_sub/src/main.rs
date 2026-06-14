use bytes::Bytes;
use shared::game_message::{GameMessage, PeerType, ClientId, Topic};

#[allow(unused)]
use log::{debug, info, warn, error};

mod pubsub;

use crate::pubsub::PubSub;

#[allow(unused)]
fn test_pub_sub_direct_calls()
{
    let mut pub_sub: PubSub = PubSub::default();


    pub_sub.subscribe(123, "aaa");
    pub_sub.subscribe(456, "aaa");
    pub_sub.subscribe(789, "bbb");

    info!("topic_subs: {:?}\n", pub_sub.topic_subs);


    pub_sub.publish("aaa", &[0x11, 0x22, 0x33]);
    PubSub::flush_peer_buffers(&mut pub_sub.subs_buffers);

    pub_sub.publish("aaa", &[0x11, 0x22, 0x33]);
    PubSub::flush_peer_buffers(&mut pub_sub.subs_buffers);
    
    pub_sub.publish("aaa", &[0x11, 0x22, 0x33, 0x44]);
    PubSub::flush_peer_buffers(&mut pub_sub.subs_buffers);
    
    pub_sub.publish("bbb", &[0x33, 0x22, 0x11]);
    PubSub::flush_peer_buffers(&mut pub_sub.subs_buffers);
    

    pub_sub.unsubscribe(456, "aaa");

    info!("topic_subs: {:?}\n", pub_sub.topic_subs);


    pub_sub.publish("aaa", &[0x11, 0x22, 0x33]);
    PubSub::flush_peer_buffers(&mut pub_sub.subs_buffers);
    
    pub_sub.publish("xyz", &[0x12,0x34,0x56,0x78,0x90]);

    info!("topics: {:?}", pub_sub.topic_data);

    return;
}

#[allow(unused)]
fn test_pub_sub_process_packets()
{
    let mut pub_sub: PubSub = PubSub::default();

    // Big Endian, so [0x01, 0x02, 0x03, 0x04] is read as 0x01_02_03_04
    //
    // 0x01_02_03_04 == 16_909_060
    
    // ---------------------------------------------------------------------------------------------------------------
    
    // Since we don't pass the peer_socket_id directly in the function we call, we need to register them first.

    let client_id = 0x0102030405060708090a0b0c0d0e0f10;
    
    pub_sub.process_received_packet(
        123_u128,
        GameMessage::Register {
            client_id: ClientId::of_player(client_id)
        }.as_bytes());

    pub_sub.process_received_packet(
        456_u128,
        GameMessage::Register {
            client_id: ClientId::of_game_server(client_id)
        }.as_bytes());

    pub_sub.process_received_packet(
        789_u128,
        GameMessage::Register {
            client_id: ClientId::of_other_server(client_id)
        }.as_bytes());

    // all 3 have the same ID, so we can test if the Peer Types works as intended
    // register complete
    // ---------------------------------------------------------------------------------------------------------------

    pub_sub.process_received_packet(
        0_u128, // doesn't matter who subscribes, what matters is who is being subscribed
        GameMessage::Subscribe {
            client_id: ClientId::of_player(client_id),
            topic: Topic("aaa".to_string()),
        }.as_bytes()
    ); // CLIENT 0x01020304... = peer_socket_id 123 (being subscribed by 000 but we don't care by who it is subscribed)

    pub_sub.process_received_packet(
        0_u128,
        GameMessage::Subscribe {
            client_id: ClientId::of_game_server(client_id),
            topic: Topic("aaa".to_string()),
        }.as_bytes()
    ); // SERVER 0x01020304... = peer_socket_id 456

    pub_sub.process_received_packet(
        0_u128,
        GameMessage::Subscribe {
            client_id: ClientId::of_other_server(client_id),
            topic: Topic("bbb".to_string()),
        }.as_bytes()
    ); // OTHER 0x01020304 = peer_socket_id 789

    // subscribes completed
    info!("topic_subs: {:?}\n", pub_sub.topic_subs);
    // ---------------------------------------------------------------------------------------------------------------

    pub_sub.process_received_packet(
        0_u128, // doesn't matter who publishes
        GameMessage::Publish {
            topic: Topic("aaa".to_string()),
            payload: vec![0x11, 0x22, 0x33],
        }.as_bytes()
    );
    PubSub::flush_peer_buffers(&mut pub_sub.subs_buffers);

    pub_sub.process_received_packet(
        0_u128,
        // same data as previous publish
        GameMessage::Publish {
            topic: Topic("aaa".to_string()),
            payload: vec![0x11, 0x22, 0x33],
        }.as_bytes()
    );
    PubSub::flush_peer_buffers(&mut pub_sub.subs_buffers);

    pub_sub.process_received_packet(
        0_u128,
        // data is longer
        GameMessage::Publish {
            topic: Topic("aaa".to_string()),
            payload: vec![0x11, 0x22, 0x33, 0x44],
        }.as_bytes()
    );
    PubSub::flush_peer_buffers(&mut pub_sub.subs_buffers);

    pub_sub.process_received_packet(
        0_u128,
        // topic is different
        GameMessage::Publish {
            topic: Topic("bbb".to_string()),
            payload: vec![0x11, 0x22, 0x33],
        }.as_bytes()
    );
    PubSub::flush_peer_buffers(&mut pub_sub.subs_buffers);

    // ---------------------------------------------------------------------------------------------------------------

    pub_sub.process_received_packet(
        0_u128,
        GameMessage::Unsubscribe {
            client_id: ClientId::of_game_server(client_id),
            topic: Topic("aaa".to_string()),
        }.as_bytes()
    );
    
    info!("topic_subs: {:?}\n", pub_sub.topic_subs);
    
    pub_sub.process_received_packet(
        0_u128,
        GameMessage::Publish {
            topic: Topic("aaa".to_string()),
            payload: vec![0x11, 0x22, 0x33],
        }.as_bytes()
    );
    PubSub::flush_peer_buffers(&mut pub_sub.subs_buffers);

    pub_sub.process_received_packet(
        0_u128,
        GameMessage::Publish {
            topic: Topic("xyz".to_string()),
            payload: vec![0x12, 0x34, 0x56, 0x78, 0x90],
        }.as_bytes()
    );
    PubSub::flush_peer_buffers(&mut pub_sub.subs_buffers);

    
    info!("topics: {:?}", pub_sub.topic_data);

    return;
}


fn main()
{
    // allow info!() logging without needing to set any environment variables
    env_logger::Builder::new().filter_level(
        log::LevelFilter::Info
        // log::LevelFilter::Debug
    ).parse_default_env().init();
    
    println!("Hello, world!\n");

    // test_pub_sub_direct_calls();
    test_pub_sub_process_packets();

    println!("\nGoodbye, world!");
}
