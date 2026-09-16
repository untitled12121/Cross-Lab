use std::{
    fs,
    net::{Ipv4Addr, SocketAddr, TcpListener},
    process,
    time::Duration,
};

use crosslab_m9_networking::netprobe::{
    NetprobePeerArgs, NetprobeRelayArgs, run_client, run_relay, run_server,
};
use tokio::{net::TcpStream, time::{sleep, timeout}};

#[tokio::test]
async fn forced_reconnect_refreshes_binding_session_and_control() {
    let bind = unused_loopback_addr();
    let relay_url = format!("http://{bind}").parse().expect("relay URL");
    let relay = tokio::spawn(run_relay(NetprobeRelayArgs::new(bind)));
    wait_for_listener(bind).await;

    let rendezvous = std::env::temp_dir().join(format!(
        "crosslab-m9-reconnect-{}.addr",
        process::id(),
    ));
    let _ = fs::remove_file(&rendezvous);
    let args = NetprobePeerArgs::new(rendezvous.clone(), relay_url);

    let (server, client) = timeout(Duration::from_secs(25), async {
        tokio::join!(run_server(args.clone()), run_client(args))
    })
    .await
    .expect("bounded reconnect netprobe");
    server.expect("netprobe server");
    let output = client.expect("netprobe client");

    for expected in [
        "phase\treconnect_verified",
        "binding_refreshed\ttrue",
        "session_refreshed\ttrue",
        "control_after_reconnect\ttrue",
    ] {
        assert!(output.lines().any(|line| line == expected), "missing {expected}");
    }

    let _ = fs::remove_file(rendezvous);
    relay.abort();
    let _ = relay.await;
}

fn unused_loopback_addr() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("ephemeral listener");
    let addr = listener.local_addr().expect("ephemeral address");
    drop(listener);
    addr
}

async fn wait_for_listener(addr: SocketAddr) {
    timeout(Duration::from_secs(5), async {
        loop {
            if TcpStream::connect(addr).await.is_ok() {
                return;
            }
            sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("owner relay listener");
}
