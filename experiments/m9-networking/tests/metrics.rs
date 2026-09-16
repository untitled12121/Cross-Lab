use std::{env, net::SocketAddr, path::PathBuf};

use crosslab_m9_networking::{
    Command, baseline,
    config::EvalConfig,
    error::EvalError,
    metrics::{Measurement, MetricKind, Report, TransportKind},
    netprobe::{NetprobePeerArgs, NetprobeRelayArgs},
};
use iroh::RelayUrl;

#[test]
fn local_command_parses_typed_sample_and_payload_limits() {
    let command = Command::parse(["local-all", "--samples", "3", "--payload-bytes", "1048576"])
        .expect("typed local command");
    let config = command.eval_config().expect("local evaluation config");

    assert_eq!(config.samples(), 3);
    assert_eq!(config.bulk_payload_bytes(), 1_048_576);
}

#[test]
fn netprobe_commands_parse_typed_routing_arguments() {
    let bind: SocketAddr = "172.30.90.1:3340".parse().expect("relay bind");
    assert_eq!(
        Command::parse(["netprobe-relay", "--bind", "172.30.90.1:3340"]),
        Ok(Command::NetprobeRelay(NetprobeRelayArgs::new(bind)))
    );

    let rendezvous = PathBuf::from("/tmp/crosslab-m9-server.addr");
    let relay_url: RelayUrl = "http://172.30.90.1:3340".parse().expect("relay URL");
    let peer_args = NetprobePeerArgs::new(rendezvous, relay_url);

    assert_eq!(
        Command::parse([
            "netprobe-server",
            "--rendezvous",
            "/tmp/crosslab-m9-server.addr",
            "--relay-url",
            "http://172.30.90.1:3340",
        ]),
        Ok(Command::NetprobeServer(peer_args.clone()))
    );
    let client = Command::parse([
        "netprobe-client",
        "--rendezvous",
        "/tmp/crosslab-m9-server.addr",
        "--relay-url",
        "http://172.30.90.1:3340",
    ])
    .expect("typed netprobe client");
    assert_eq!(client, Command::NetprobeClient(peer_args));
    assert_eq!(client.eval_config(), None);
}

#[tokio::test]
async fn netprobe_commands_are_rejected_by_local_runner() {
    let command = Command::parse(["netprobe-relay", "--bind", "172.30.90.1:3340"])
        .expect("typed netprobe relay");

    assert_eq!(baseline::run_local(command).await, Err(EvalError::InvalidCommand));
}

#[test]
fn report_tsv_records_reproducibility_header() {
    let config = EvalConfig::test();
    let report = Report::for_run(
        config,
        vec![Measurement::new(
            TransportKind::Quinn,
            MetricKind::ProtectedConnectMicros,
            0,
            42,
        )],
    );
    let output = report.to_tsv();

    for expected in [
        format!("# os={}", env::consts::OS),
        format!("# arch={}", env::consts::ARCH),
        "# rust=1.98.1".to_owned(),
        format!("# samples={}", config.samples()),
        format!("# payload_bytes={}", config.bulk_payload_bytes()),
        "# quinn=0.11.11".to_owned(),
        "# iroh=1.2.0".to_owned(),
    ] {
        assert!(
            output.lines().any(|line| line == expected),
            "missing {expected}"
        );
    }
    assert!(output.contains("transport\tmetric\tsample\tvalue\n"));
    assert!(output.contains("quinn\tprotected_connect_us\t0\t42\n"));
}
