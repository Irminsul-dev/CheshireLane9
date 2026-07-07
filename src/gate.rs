use std::sync::atomic::{AtomicU32, Ordering};

use anyhow::Result;
use proto::p10::{Cs10022, Sc10023};
use proto::CmdID;
use rand::RngCore;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::config::CONFIG;
use crate::crypto::md5_with_salt;
use crate::database::Database;
use crate::game::PlayerRuntime;
use crate::packet::Packet;

static NEXT_CONV: AtomicU32 = AtomicU32::new(1);

#[derive(Clone)]
pub struct GateState {
    db: Database,
    runtime: PlayerRuntime,
}

pub async fn serve(db: Database, runtime: PlayerRuntime) -> Result<()> {
    let listener = TcpListener::bind(CONFIG.gate_addr).await?;
    let state = GateState { db, runtime };
    tracing::info!("gate listening on {}", CONFIG.gate_addr);

    loop {
        let (stream, addr) = listener.accept().await?;
        let state = state.clone();
        tracing::debug!("gate connection from {addr}");
        tokio::spawn(async move {
            if let Err(err) = handle_client(stream, state).await {
                tracing::error!("gate client failed: {err}");
            }
        });
    }
}

async fn handle_client(mut stream: TcpStream, state: GateState) -> Result<()> {
    stream.set_nodelay(true)?;
    let _conv = NEXT_CONV.fetch_add(1, Ordering::Relaxed);
    let _token = rand::thread_rng().next_u32();
    let mut uid = None;
    let mut buffer = Vec::new();
    let mut previous_packet_id = None;
    let mut chunk = [0u8; 4096];

    loop {
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            return Ok(());
        }
        buffer.extend_from_slice(&chunk[..n]);

        while let Some(packet) = Packet::split_from_after(&mut buffer, previous_packet_id) {
            let packet = packet?;
            if packet.id != 0 {
                previous_packet_id = Some(packet.id);
            }

            for rsp in handle_packet(packet, &state, &mut uid).await? {
                tracing::debug!(
                    service = "gate",
                    path = "send",
                    cmd_id = rsp.cmd_id,
                    packet_id = rsp.id,
                    payload_len = rsp.data.len(),
                    "gate response"
                );
                stream.write_all(&rsp.to_bytes()).await?;
            }
        }
    }
}

async fn handle_packet(
    packet: Packet,
    state: &GateState,
    uid: &mut Option<u32>,
) -> Result<Vec<Packet>> {
    let current_uid = *uid;
    let path = if packet.cmd_id == Cs10022::CMD_ID {
        "login"
    } else if current_uid.is_some() {
        "game"
    } else {
        "pre-login"
    };
    tracing::debug!(
        service = "gate",
        path,
        proto = crate::game::request_proto_name(packet.cmd_id),
        cmd_id = packet.cmd_id,
        packet_id = packet.id,
        uid = ?current_uid,
        "gate packet"
    );
    if path == "game" {
        tracing::debug!(
            service = "gate",
            path,
            cmd_id = packet.cmd_id,
            packet_id = packet.id,
            wire_len = packet.raw_len(),
            length = packet.length,
            flag = packet.flag,
            payload_len = packet.data.len(),
            raw = %packet.raw_hex_prefix(48),
            "gate packet raw"
        );
    }

    if packet.cmd_id == Cs10022::CMD_ID {
        return handle_login(packet, state, uid).await;
    }

    let Some(uid) = *uid else {
        tracing::warn!("dropping cmd {} before login", packet.cmd_id);
        return Ok(Vec::new());
    };

    state.runtime.handle_packet(uid, packet).await
}

async fn handle_login(
    packet: Packet,
    state: &GateState,
    uid_slot: &mut Option<u32>,
) -> Result<Vec<Packet>> {
    let Some(req) = packet.decode::<Cs10022>() else {
        return Ok(Vec::new());
    };

    let mut result = 1;
    let mut user_id = 0;
    let md5 = md5_with_salt(&req.server_ticket, &CONFIG.salt);

    if md5 == req.check_key {
        if let Some(account) = state.db.get_account(req.account_id).await? {
            user_id = account.uid as u32;
            let player = state.db.get_player_row(req.account_id).await?;
            result = if player.as_ref().is_some_and(|row| row.is_banned != 0) {
                17
            } else {
                0
            };
        }
    }

    if result != 0 {
        return Ok(vec![Packet::encode(
            &Sc10023 {
                db_load: None,
                result,
                user_id,
                server_ticket: req.server_ticket,
                server_load: None,
            },
            packet.id,
        )]);
    }

    *uid_slot = Some(req.account_id);
    state.runtime.ensure_player(req.account_id).await?;
    state.runtime.handle_packet(req.account_id, packet).await
}
