
#[allow(unused)]
use log::{debug, info, warn, error};

use std::collections::HashMap;
use shared::{ClientId, game_message::GameMessage};
use bytes::{
    BufMut, 
    BytesMut,
    Bytes,
};

// -------------------------------------------------------------------------------------------------------------------

pub fn u8_slice_to_hex_string(bytes: &[u8]) -> String
{
    return bytes.iter().map(|b| { format!("{:02x}", *b) }).collect();
}

// -------------------------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------------------------

/**
    ### PubSubMsgType
    First Byte of packets sent or received by the PubSub
 */
#[allow(unused)]
#[derive(Clone, Copy)]
pub enum PubSubMsgType
{
    None,
    Subscribe,   // Peer -> PubSub
    Unsubscribe, // Peer -> PubSub
    Publish,     // Peer -> PubSub
    Broadcast,   // PubSub -> Peer
    ClientInput, // Client -> PubSub
    Register,    // Peer -> PubSub
}


// -------------------------------------------------------------------------------------------------------------------

use game_sockets::{GameConnection, GameStream, GamePeer};

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub struct PeerSocketId(pub GameConnection, pub GameStream);

#[derive(Default)]
pub struct PubSub
{
    pub subs_peer_sockets: HashMap<ClientId, PeerSocketId>,

    pub topic_data: HashMap<String, Vec<u8>>,
    pub topic_subs: HashMap<String, Vec<PeerSocketId>>,

    pub subs_buffers: HashMap<PeerSocketId, BytesMut>
}

// -------------------------------------------------------------------------------------------------------------------

impl PubSub
{
    /**
        Gets the PeerSocketId (used by GameSockets to actually communicate with a peer) from a PubSubPeerType (u8) and a PeerId (u32)
     */
    #[allow(unused)]
    pub fn get_peer_socket_id(&self, client_id: ClientId) -> Option<PeerSocketId>
    {
        return self.subs_peer_sockets.get(&client_id).cloned();
    }

    /**
        Sets a PeerSocketId (used by GameSockets to actually communicate with a peer) that corresponds to a PubSubPeerType (u8) and a PeerId (u32)
    */
    #[allow(unused)]
    pub fn set_peer_socket_id(&mut self, client_id: ClientId, peer_socket_id: PeerSocketId)
    {
        let insert_result = self.subs_peer_sockets.insert(client_id, peer_socket_id);
        if insert_result.is_some()
        {
            warn!(
                "set_peer_socket_id() : (client_id:{:?}) was already present in subs_peer_sockets, with peer_socket_id = {:?} (it is now set to {:?})", 
                client_id, insert_result.unwrap(), peer_socket_id
            );
        }
        else
        {
            info!("set_peer_socket_id() : added peer_socket_id {:?} as type {:?}", peer_socket_id ,client_id);
        }
        return;
    }

    /**
        Subscribes a PeerSocketId to a Topic
     */
    pub fn subscribe(&mut self, peer_socket_id: PeerSocketId, topic: &str)
    {
        match self.topic_subs.get_mut(topic)
        {
            None => 
            {
                self.topic_subs.insert(topic.to_owned(), vec![peer_socket_id]);
                info!("subscribe() : {:?} is now subscribed to topic \"{:?}\"", peer_socket_id, topic);
            }

            Some(subs_vec) => 
            {
                if subs_vec.contains(&peer_socket_id) == false
                {
                    // insert in a sorted way, so we can test contains() in O(log(n)) instead of O(n) ?
                    // (but then in unsubscribe, we can't use swap_remove() in O(1), we would need to remove() in O(n) there) ?

                    subs_vec.push(peer_socket_id);
                    info!("subscribe() : {:?} is now subscribed to topic \"{:?}\"", peer_socket_id, topic);
                }
                else
                {
                    warn!("subscribe() : {:?} is already subscribed to topic \"{:?}\"", peer_socket_id, topic);
                }
            }
        }
        return;
    }

    /**
        Unsubscribes a PeerSocketId to a Topic
    */
    pub fn unsubscribe(&mut self, peer_socket_id: PeerSocketId, topic: &str)
    {
        match self.topic_subs.get_mut(topic)
        {
            Some(subs_vec) => 
            {
                let Some(found_id) = subs_vec.iter().position(|val| {*val == peer_socket_id} )
                else
                {
                    warn!("unsubscribe() : {:?} wasn't subscribed to topic \"{:?}\"", peer_socket_id, topic);
                    return;
                };
                
                // subs_vec[found_id] == peer_socket_id
                subs_vec.swap_remove(found_id);

                info!("unsubscribe() : {:?} is now unsubscribed to topic \"{:?}\"", peer_socket_id, topic);
            }

            None => 
            {
                warn!("unsubscribe() : {:?} wasn't subscribed to topic \"{:?}\"", peer_socket_id, topic);
            }
        }
        return;
    }

    /**
        Sets the data associated with a Topic, and calls broadcast() on that topic if the data is different from the previously stored data
    */
    pub fn publish(&mut self, topic: &str, data: &[u8])
    {
        // HashMap<K,V>::insert(k,v) : "if the map did have this key present, the value is updated, and the old value is returned."

        let insert_result = self.topic_data.insert(topic.to_owned(), data.to_owned());

        if let Some(old_data) = insert_result // if there were already data, 
        {
            if old_data == data // AND it was the same data, 
            {
                info!("publish() : topic \"{}\" : data published was already present", topic);
                return; // we do nothing
            }
        }
        // else :
        // either there wasn't any data before this call, or there were but it was different data

        info!("publish() : topic \"{}\" : data published : 0x{}", topic, u8_slice_to_hex_string(data));
        self.broadcast(topic);
        return;
    }

    /**
        For all subscribed peers : adds a "subpacket" containing a topic and its new data, inside their buffer (by calling add_subpacket_for_peer).
        
        **Don't forget to flush the buffers to actually send them through the network, otherwise you would just fill an internal buffer !**
     */
    pub fn broadcast(&mut self, topic: &str)
    {
        let Some(subs) = self.topic_subs.get(topic)
        else
        {
            debug!("broadcast() : no one is subscribed to topic \"{}\" : not broadcasting anything", topic);
            return;
        };
        debug!("broadcast() : subscribed to topic \"{}\" : {:?}", topic, subs);

        let data_ref = self.topic_data.get(topic);
        let data_length: usize = if let Some(data) = data_ref { data.len() } else { 0_usize };

        /*
            packet = [ u16,  [u8;...],     u16, [u8;...] ]
                       ^      ^            ^     ^
             topic_length  topic  data_length  data
        */
        let topic_size: usize = size_of::<u16>() + topic.len();
        let data_size: usize = size_of::<u16>() + data_length;

        let mut packet = BytesMut::with_capacity(topic_size + data_size);

        if topic.len() >= u16::MAX as usize { panic!("PubSub::broadcast() : topic.len() >= {}", u16::MAX as usize); }
        packet.put_u16(topic.len() as u16);
        packet.put_slice(topic.as_bytes());

        if data_length >= u16::MAX as usize { panic!("PubSub::broadcast() : data_length >= {}", u16::MAX as usize); }
        packet.put_u16(data_length as u16);
        if data_length > 0
        {
            packet.put_slice(data_ref.unwrap());
        }

        let packet = packet.freeze();

        debug!("broadcast() : packet=0x{}", u8_slice_to_hex_string(&packet));

        for peer_socket_id in subs
        {
            // We only need a mutable reference to the subs buffer, not to the whole PubSub struct.
            // Moreover, if we took the whole PubStruct, we could not compile the program, since we could, in theory,
            // modify the topic_data inside this function, 
            // while we still hold and use (in the next iteration of the loop) non mutable references to the topic_data
            
            Self::add_subpacket_for_peer(&mut self.subs_buffers, *peer_socket_id, &packet);
        }
        return;
    }

    /**
        For the provided peer only, adds inside their buffer a "subpacket".
        If their buffer was empty, adds the necessary header so that the buffer is actually a whole packet containing the right header, followed by the "subpackets"
        
        buffer = [header, subpacket_1, subpacket_2, ...]
     */
    fn add_subpacket_for_peer(subs_buffers: &mut HashMap<PeerSocketId, BytesMut>, peer_socket_id:  PeerSocketId, bytes: &Bytes)
    {
        let get_result = subs_buffers.get_mut(&peer_socket_id);
        if let Some(packet) = get_result
        {
            packet.put_slice(bytes);
        }
        else // there isn't any buffer
        {
            let mut new_packet = BytesMut::with_capacity(bytes.len());

            new_packet.put_slice(bytes);
            
            subs_buffers.insert(peer_socket_id, new_packet);
        }
        debug!("add_subpacket_for_peer() : peer {:?}: buffer=0x{}", peer_socket_id, u8_slice_to_hex_string(subs_buffers.get(&peer_socket_id).unwrap()));
        return;
    }

    /**
        For all peers, we send them their buffers through the network
     */
    pub fn flush_peer_buffers(&mut self, peer : &mut GamePeer)
    {
        for (peer_socket_id, packet) in self.subs_buffers.iter()
        {
            if packet.len() > 0_usize
            {
                // Pelle:
                // Reconstruire l'entiereté du packet Broadcast était un peu lourd dans ta version
                // J'ai préféré le construire qu'une fois ici
                //
                // Peut-être vider le contenu de BytesMut ?
                let payload = packet.clone().to_vec();                
                let mut packet_bytes = BytesMut::new();
                GameMessage::Broadcast{payload}.append_bytes(&mut packet_bytes);
                           
                let res = peer.send(&peer_socket_id.0, &peer_socket_id.1, packet_bytes.freeze());
                match res {
                    Ok(()) => {
                        info!(
                            "flush_peer_buffers() : sent {:?} to {}", 
                            peer_socket_id, u8_slice_to_hex_string(&packet)
                        ); 
                    }
                    _ => {
                        error!("flush_peer_buffers() : {:?}", res);
                    }                    
                }               
            }
        }
        self.subs_buffers.clear();
        return;
    }

    /**
        "Unserializes" received packets and calls related functions with the wanted parameters
     */
    pub fn process_received_packet(&mut self, peer_socket_id: PeerSocketId, bytes: Bytes)
    {
        if let Some(msg) = GameMessage::from_bytes(&mut bytes.clone()) {
            match msg {
                GameMessage::Register{client_id} => {
                    self.set_peer_socket_id(client_id, peer_socket_id);
                }
                GameMessage::Subscribe{client_id, topic} => {
                    if let Some(peer_to_sub) = self.get_peer_socket_id(client_id) {
                        self.subscribe(peer_to_sub, topic.as_str());
                    }
                    else
                    {
                        error!("process_received_packet() : peer to subscribe couldn't be found ({:?})", client_id);
                    };
                }
                GameMessage::Unsubscribe{client_id, topic} => {
                    if let Some(peer_to_sub) = self.get_peer_socket_id(client_id) {
                        self.unsubscribe(peer_to_sub, topic.as_str());
                    }
                    else
                    {
                        error!("process_received_packet() : peer to unsubscribe couldn't be found ({:?})", client_id);
                    };
                }
                GameMessage::Publish{topic, payload} => {
                    // TODO : check for authority (client can't set some entity's position, only a server can publish its status (CPU usage,...), ...)
                    self.publish(topic.as_str(), &payload);
                }
                GameMessage::ClientInput{client_id:_, input:_} => {
                    warn!("process_received_packet() : received a ClientInput");
                }
                _ => {
                    error!("process_received_packet() : Unhandled message : {:?}", msg);
                }
            }
        }
        else {
            error!("process_received_packet() : packet is invalid ({:?})", bytes);
        }
    }
    
}
