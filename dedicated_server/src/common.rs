use std::net::{SocketAddr, IpAddr::{V4, V6}, Ipv4Addr, Ipv6Addr, IpAddr};
use bytes::{Buf, BufMut, Bytes, BytesMut};

pub const SECONDS_BETWEEN_HEARTBEATS : f32 = 5.0;

pub struct Heartbeat {
    pub id : String,
    pub addr : SocketAddr,
    pub zone : String,
    pub player_count : usize,
    pub is_full : bool,
}

impl Heartbeat {
    pub fn to_bytes(&self) -> Bytes {
        // id (uuid v4): 16 octets
        // addr est de taille 4 octets ou 16 octets. On ajoute un booléen (1 octet) pour dire si on est ipv4 ou non
        // port (u16): 2 octets
        // zone : arbitraire, on range length de longueur 8 puis la donnée
        // player_count, 8 octets
        // On vérifie quand même
        
        let len = 16 + 1 + 16 + 2 + 32 + 8;
        let mut res = BytesMut::with_capacity(len);

        let Self{id, addr, zone, player_count, is_full} = self;

        assert_eq!(id.len(), 16);
        res.put_slice(id.as_bytes());
        let bools = (addr.is_ipv4() as u8) | (*is_full as u8) << 1;
        res.put_u8(bools);

        match addr.ip() {
            V4(ipv4) => { res.put_slice(&ipv4.octets()); }
            V6(ipv6) => { res.put_slice(&ipv6.octets()); }
        }
        res.put_u16(addr.port());

        res.put_u64(zone.len() as u64);
        res.put_slice(zone.as_bytes());
        res.put_u64(*player_count as u64);
        
        res.freeze()
    }

    pub fn from_bytes(mut data : Bytes) -> Option<Self> {
        let mut id_bytes = [0; 16];
        data.copy_to_slice(&mut id_bytes);
        let res_id = String::from_utf8(id_bytes.to_vec());

        let Ok(id) = res_id else { return None; };

        let bools = data.get_u8();
        let is_ipv4 = bools & 1 == 1;
        let is_full = (bools >> 1) == 1;
        let addr : SocketAddr;
        if is_ipv4 {
            let mut ipv4_bytes = [0; 4];
            data.copy_to_slice(&mut ipv4_bytes);
            let port = data.get_u16();

            addr = SocketAddr::new(IpAddr::V4(
                Ipv4Addr::new(ipv4_bytes[0], ipv4_bytes[1], ipv4_bytes[2], ipv4_bytes[3])
            ), port);
        }
        else {
            let mut ipv6_bytes = [0; 16];
            data.copy_to_slice(&mut ipv6_bytes);
            let port = data.get_u16();

            addr = SocketAddr::new(IpAddr::V6(
                Ipv6Addr::from_bits(u128::from_be_bytes(ipv6_bytes))
            ), port);
        }

        let str_len = data.get_u64();
        let mut bytes_vec = Vec::with_capacity(str_len as usize);
        data.copy_to_slice(&mut bytes_vec);

        let res_zone = String::from_utf8(bytes_vec);
        let Ok(zone) = res_zone else { return None; };
        let player_count = data.get_u64();

        Some(Self {id, addr, zone, player_count: player_count as usize, is_full})
    }
}
