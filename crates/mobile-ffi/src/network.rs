use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

pub(crate) fn socket_addr(address: Vec<u8>, port: i32, scope_id: i32) -> Option<SocketAddr> {
    let port = u16::try_from(port).ok().filter(|port| *port != 0)?;
    let scope_id = u32::try_from(scope_id).ok()?;

    match address.as_slice() {
        [a, b, c, d] => Some(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::new(*a, *b, *c, *d),
            port,
        ))),
        bytes if bytes.len() == 16 => {
            let bytes: [u8; 16] = bytes.try_into().ok()?;
            Some(SocketAddr::V6(SocketAddrV6::new(
                Ipv6Addr::from(bytes),
                port,
                0,
                scope_id,
            )))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_supports_ipv4_and_scoped_ipv6() {
        assert_eq!(
            socket_addr(vec![127, 0, 0, 1], 443, 0).unwrap(),
            "127.0.0.1:443".parse().unwrap()
        );

        let v6 = socket_addr(
            vec![0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
            443,
            7,
        )
        .unwrap();
        let SocketAddr::V6(v6) = v6 else {
            panic!("expected IPv6 route");
        };
        assert_eq!(v6.scope_id(), 7);
    }

    #[test]
    fn route_rejects_invalid_shape_and_zero_port() {
        assert!(socket_addr(vec![127, 0, 0, 1], 0, 0).is_none());
        assert!(socket_addr(vec![127, 0, 0], 443, 0).is_none());
        assert!(socket_addr(vec![127, 0, 0, 1], 443, -1).is_none());
    }
}
