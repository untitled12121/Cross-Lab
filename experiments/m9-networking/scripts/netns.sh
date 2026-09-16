#!/usr/bin/env bash
set -euo pipefail

readonly ROUTER_A=cl-m9-ra
readonly ROUTER_B=cl-m9-rb
readonly PEER_A=cl-m9-a
readonly PEER_B=cl-m9-b
readonly BRIDGE=cl-m9-br
readonly RELAY_URL=http://172.30.90.1:3340

usage() {
    echo "usage: $0 <m9-networking-binary> <output-tsv>" >&2
    exit 2
}

fail() {
    echo "netns: $*" >&2
    exit 1
}

[[ $# -eq 2 ]] || usage
[[ ${EUID:-$(id -u)} -eq 0 ]] || fail "must run as root"
[[ -x "$1" ]] || fail "binary is not executable: $1"

BIN=$(readlink -f "$1")
OUTPUT=$2

for command in ip nft sysctl awk grep readlink mktemp sleep; do
    command -v "$command" >/dev/null 2>&1 || fail "missing required command: $command"
done

existing_namespaces=$(ip netns list | awk '{print $1}')
for namespace in "$ROUTER_A" "$ROUTER_B" "$PEER_A" "$PEER_B"; do
    if grep -Fxq "$namespace" <<<"$existing_namespaces"; then
        fail "planned namespace already exists: $namespace"
    fi
done

if ip link show cl-m9-br >/dev/null 2>&1; then
    fail "planned bridge already exists: cl-m9-br"
fi

for link in m9-ra-ext m9-ra-ext-ns m9-rb-ext m9-rb-ext-ns m9-ra-int m9-a-eth m9-rb-int m9-b-eth; do
    if ip link show "$link" >/dev/null 2>&1; then
        fail "planned link already exists: $link"
    fi
done

WORKDIR=$(mktemp -d /tmp/crosslab-m9-netns.XXXXXX)
RENDEZVOUS="$WORKDIR/server.addr"
RELAY_LOG="$WORKDIR/relay.log"
SERVER_LOG="$WORKDIR/server.log"
CLIENT_LOG="$WORKDIR/client.log"
relay_pid=
server_pid=
client_pid=

dump_log() {
    local label=$1
    local path=$2
    if [[ -f "$path" ]]; then
        echo "=== $label ===" >&2
        cat "$path" >&2
    fi
}

cleanup() {
    local status=$?
    set +e

    if ((status != 0)); then
        dump_log m9-relay-log "$RELAY_LOG"
        dump_log m9-server-log "$SERVER_LOG"
        dump_log m9-client-log "$CLIENT_LOG"
    fi

    for pid in "$client_pid" "$server_pid" "$relay_pid"; do
        if [[ -n "$pid" ]]; then
            kill "$pid" 2>/dev/null || true
        fi
    done
    for pid in "$client_pid" "$server_pid" "$relay_pid"; do
        if [[ -n "$pid" ]]; then
            wait "$pid" 2>/dev/null || true
        fi
    done

    for router in "$ROUTER_A" "$ROUTER_B"; do
        if ip netns list | awk '{print $1}' | grep -Fxq "$router"; then
            ip netns exec "$router" nft delete table ip cl_m9_gate 2>/dev/null || true
            ip netns exec "$router" nft delete table ip cl_m9_nat 2>/dev/null || true
        fi
    done

    for namespace in "$PEER_A" "$PEER_B" "$ROUTER_A" "$ROUTER_B"; do
        ip netns delete "$namespace" 2>/dev/null || true
    done
    ip link delete "$BRIDGE" 2>/dev/null || true
    rm -rf "$WORKDIR"
    return "$status"
}
trap cleanup EXIT INT TERM

ip netns add "$ROUTER_A"
ip netns add "$ROUTER_B"
ip netns add "$PEER_A"
ip netns add "$PEER_B"

ip link add "$BRIDGE" type bridge
ip addr add 172.30.90.1/24 dev "$BRIDGE"
ip link set "$BRIDGE" up

setup_router() {
    local router=$1
    local peer=$2
    local root_ext=$3
    local ns_ext=$4
    local router_int=$5
    local peer_int=$6
    local transit_addr=$7
    local router_addr=$8
    local peer_addr=$9
    local gateway=${router_addr%/*}

    ip link add "$root_ext" type veth peer name "$ns_ext"
    ip link set "$ns_ext" netns "$router"
    ip link set "$root_ext" master "$BRIDGE"
    ip link set "$root_ext" up

    ip -n "$router" link set "$ns_ext" name ext0
    ip -n "$router" link set lo up
    ip -n "$router" addr add "$transit_addr" dev ext0
    ip -n "$router" link set ext0 up

    ip link add "$router_int" type veth peer name "$peer_int"
    ip link set "$router_int" netns "$router"
    ip link set "$peer_int" netns "$peer"

    ip -n "$router" link set "$router_int" name int0
    ip -n "$router" addr add "$router_addr" dev int0
    ip -n "$router" link set int0 up

    ip -n "$peer" link set "$peer_int" name eth0
    ip -n "$peer" link set lo up
    ip -n "$peer" addr add "$peer_addr" dev eth0
    ip -n "$peer" link set eth0 up
    ip -n "$peer" route add default via "$gateway"

    ip netns exec "$router" sysctl -q -w net.ipv4.ip_forward=1 >/dev/null
    ip netns exec "$router" nft -f - <<'NFT'
table ip cl_m9_nat {
    chain postrouting {
        type nat hook postrouting priority srcnat; policy accept;
        oifname "ext0" masquerade
    }
}
NFT
}

setup_router "$ROUTER_A" "$PEER_A" m9-ra-ext m9-ra-ext-ns m9-ra-int m9-a-eth \
    172.30.90.2/24 10.90.1.1/24 10.90.1.2/24
setup_router "$ROUTER_B" "$PEER_B" m9-rb-ext m9-rb-ext-ns m9-rb-int m9-b-eth \
    172.30.90.3/24 10.90.2.1/24 10.90.2.2/24

block_direct_udp() {
    local router
    for router in "$ROUTER_A" "$ROUTER_B"; do
        ip netns exec "$router" nft -f - <<'NFT'
table ip cl_m9_gate {
    chain forward {
        type filter hook forward priority filter; policy accept;
        ip protocol udp drop
    }
}
NFT
    done
}

allow_direct_udp() {
    local router
    for router in "$ROUTER_A" "$ROUTER_B"; do
        ip netns exec "$router" nft delete table ip cl_m9_gate
    done
}

block_direct_udp

"$BIN" netprobe-relay --bind 172.30.90.1:3340 >"$RELAY_LOG" 2>&1 &
relay_pid=$!

ip netns exec "$PEER_B" "$BIN" netprobe-server \
    --rendezvous "$RENDEZVOUS" \
    --relay-url "$RELAY_URL" >"$SERVER_LOG" 2>&1 &
server_pid=$!

for ((attempt = 0; attempt < 200; attempt++)); do
    [[ -s "$RENDEZVOUS" ]] && break
    kill -0 "$server_pid" 2>/dev/null || fail "netprobe server exited before rendezvous"
    sleep 0.05
done
[[ -s "$RENDEZVOUS" ]] || fail "timed out waiting for rendezvous"

ip netns exec "$PEER_A" "$BIN" netprobe-client \
    --rendezvous "$RENDEZVOUS" \
    --relay-url "$RELAY_URL" >"$CLIENT_LOG" 2>&1 &
client_pid=$!

for ((attempt = 0; attempt < 400; attempt++)); do
    grep -Fq $'phase\trelay_verified' "$CLIENT_LOG" && break
    kill -0 "$client_pid" 2>/dev/null || fail "netprobe client exited before relay verification"
    sleep 0.05
done
grep -Fq $'phase\trelay_verified' "$CLIENT_LOG" || fail "timed out waiting for relay verification"

allow_direct_udp

wait "$client_pid"
client_pid=
wait "$server_pid"
server_pid=

cat "$CLIENT_LOG" >"$OUTPUT"
