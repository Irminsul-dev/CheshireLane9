use anyhow::Result;
use cheshire_server_core::config::Config;
use cheshire_server_core::crypto::md5_with_salt;
use cheshire_server_core::packet::Packet;
use cheshire_server_core::time;
use cheshire_server_proto::p10::{
    Cs10018, Cs10020, Cs10800, Sc10019, Sc10021, Sc10801, Serverinfo,
};
use cheshire_server_proto::CmdID;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinSet;

#[derive(Clone)]
struct DispatchData {
    gateway_ip: String,
    gateway_port: u16,
    version: Vec<String>,
    servers: Vec<Serverinfo>,
}

pub async fn serve(config: Config) -> Result<()> {
    let listen_addr = config.dispatch_addr;
    let data = DispatchData {
        gateway_ip: config.dispatch_ip,
        gateway_port: config.dispatch_port,
        version: config.dispatch_version,
        servers: config.dispatch_servers,
    };

    let listener = TcpListener::bind(listen_addr).await?;
    tracing::info!("dispatch listening on {listen_addr}");
    let mut clients = JoinSet::new();

    loop {
        tokio::select! {
            connection = listener.accept() => {
                let (stream, addr) = connection?;
                let data = data.clone();
                tracing::debug!("dispatch connection from {addr}");
                clients.spawn(async move {
                    if let Err(err) = handle_client(stream, data).await {
                        tracing::error!("dispatch client failed: {err}");
                    }
                });
            }
            Some(result) = clients.join_next(), if !clients.is_empty() => {
                if let Err(err) = result {
                    tracing::error!("dispatch client task failed: {err}");
                }
            }
        }
    }
}

async fn handle_client(mut stream: TcpStream, data: DispatchData) -> Result<()> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];

    loop {
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            return Ok(());
        }
        buffer.extend_from_slice(&chunk[..n]);

        while let Some(packet) = Packet::split_from(&mut buffer) {
            if let Some(rsp) = handle_packet(packet?, &data) {
                stream.write_all(&rsp.to_bytes()).await?;
            }
        }
    }
}

fn handle_packet(packet: Packet, data: &DispatchData) -> Option<Packet> {
    tracing::debug!(
        service = "dispatch",
        path = "packet",
        proto = request_proto_name(packet.cmd_id),
        cmd_id = packet.cmd_id,
        packet_id = packet.id,
        "dispatch packet"
    );

    match packet.cmd_id {
        Cs10800::CMD_ID => Some(Packet::encode(
            &Sc10801 {
                gateway_ip: data.gateway_ip.clone(),
                gateway_port: data.gateway_port as u32,
                url: format!("http://{}", data.gateway_ip),
                version: data.version.clone(),
                proxy_ip: Some(data.gateway_ip.clone()),
                proxy_port: Some(data.gateway_port as u32),
                is_ts: 0,
                timestamp: time::now_timestamp_s() as u32,
                monday_0oclock_timestamp: 1_606_114_800,
                cdn_list: vec![],
            },
            packet.id,
        )),
        Cs10020::CMD_ID => packet.decode::<Cs10020>().map(|req| {
            let hash = md5_with_salt(&req.arg1);
            let result = u32::from(hash != req.check_key);
            Packet::encode(
                &Sc10021 {
                    result,
                    serverlist: data.servers.clone(),
                    account_id: req.arg2.and_then(|v| v.parse().ok()).unwrap_or_default(),
                    server_ticket: req.arg3.unwrap_or_default(),
                    notice_list: vec![],
                    device: req.device,
                    limit_server_ids: vec![],
                },
                packet.id,
            )
        }),
        Cs10018::CMD_ID => Some(Packet::encode(
            &Sc10019 {
                serverlist: data.servers.clone(),
            },
            packet.id,
        )),
        _ => {
            tracing::debug!("dispatch unhandled cmd {}", packet.cmd_id);
            None
        }
    }
}

fn request_proto_name(cmd_id: u16) -> &'static str {
    match cmd_id {
        Cs10800::CMD_ID => "p10.Cs10800",
        Cs10020::CMD_ID => "p10.Cs10020",
        Cs10018::CMD_ID => "p10.Cs10018",
        _ => "unknown",
    }
}
