const NETNS_SCRIPT: &str = include_str!("../scripts/netns.sh");

#[test]
fn netns_script_uses_fixed_owned_topology_and_refuses_collisions() {
    for required in [
        "cl-m9-ra",
        "cl-m9-rb",
        "cl-m9-a",
        "cl-m9-b",
        "cl-m9-br",
        "172.30.90.1/24",
        "172.30.90.2/24",
        "172.30.90.3/24",
        "10.90.1.1/24",
        "10.90.2.1/24",
        "ip netns list",
        "ip link show cl-m9-br",
    ] {
        assert!(NETNS_SCRIPT.contains(required), "missing {required}");
    }
}

#[test]
fn netns_script_owns_forwarding_nat_path_gate_and_cleanup() {
    for required in [
        "net.ipv4.ip_forward=1",
        "table ip cl_m9_nat",
        "oifname \"ext0\" masquerade",
        "netprobe-relay",
        "--bind 172.30.90.1:3340",
        "netprobe-server",
        "netprobe-client",
        "udp drop",
        "$'phase\\trelay_verified'",
        "trap cleanup EXIT INT TERM",
    ] {
        assert!(NETNS_SCRIPT.contains(required), "missing {required}");
    }

    let block = NETNS_SCRIPT.find("udp drop").expect("direct UDP block");
    let allow = NETNS_SCRIPT[block..]
        .find("nft delete table ip cl_m9_gate")
        .map(|offset| block + offset)
        .expect("direct UDP unblock");
    assert!(
        block < allow,
        "direct UDP must be blocked before it is allowed"
    );
}
