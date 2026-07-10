pub mod player_info;

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use proto::common::{Collectioninfo, Displayinfo, Kvdata};
use proto::p10::{Cs10022, Cs10024, Cs10100, Sc10023, Sc10025, Sc10101};
use proto::p11::{
    Cs11001, Cs11011, Cs11017, Cs11603, Cs11701, Cs11705, Cs11710, Cs11722, Cs11751, InsMessage,
    Noticeinfo, Sc11000, Sc11002, Sc11012, Sc11018, Sc11200, Sc11210, Sc11300, Sc11604, Sc11702,
    Sc11706, Sc11711, Sc11723, Sc11752,
};
use proto::p12::{Cs12202, Cs12299, Cs12406, Sc12010, Sc12024, Sc12031, Sc12203, Sc12407};
use proto::p13::{Cs13505, Sc13002, Sc13201, Sc13506};
use proto::p15::{Cs15008, Cs15300, Sc15009};
use proto::p16::{Cs16104, Sc16105, Sc16200};
use proto::p18::{Cs18001, Sc18002};
use proto::p19::{Cs19009, Sc19010};
use proto::p20::{Cs20007, Sc20001, Sc20008, Sc20101, Sc20201};
use proto::p21::Sc21536;
use proto::p22::Sc22300;
use proto::p24::{Cs24020, Sc24021};
use proto::p25::{Commanderhomeslot, Cs25026, Sc25027};
use proto::p26::{Cs26101, Sc26102, Sc26120};
use proto::p27::{
    ChildAttr, ChildFavor, ChildInfo, ChildTask, ChildTime, Cs27000, Cs27010, Sc27001, Sc27011,
};
use proto::p28::Sc28000;
use proto::p29::{
    Cs29001, Sc29002, Tbbenefit, Tbbf, Tbdisplay, Tbfsm, Tbfsmcache, Tbinfo, Tbpermanent, Tbplan,
    Tbres, Tbround, Tbsite, Tbtalent,
};
use proto::p30::{Sc30001, Sc30101};
use proto::p34::{Cs34001, Cs34501, MetaShipInfo, Sc34002, Sc34502};
use proto::p50::{Cs50102, PlayerInfo as ChatPlayerInfo, Sc50000, Sc50101};
use proto::p60::{Cs60037, Cs60102, GuildBaseInfo, GuildInfo, Sc60000, Sc60103, UserGuildInfo};
use proto::p62::{Cs62100, Sc62101};
use proto::p63::{Cs63317, Sc63000, Sc63100, Sc63318};
use proto::p64::Sc64000;
use proto::CmdID;
use tokio::sync::Mutex;

use crate::config::CONFIG;
use crate::database::Database;
use crate::packet::Packet;
use crate::time;
use player_info::PlayerInfo;

const MONDAY_0_TIMESTAMP: u32 = 1_606_114_800;

#[derive(Clone)]
pub struct PlayerRuntime {
    db: Database,
    players: Arc<Mutex<HashMap<u32, PlayerSession>>>,
}

struct PlayerSession {
    info: PlayerInfo,
    dirty: bool,
}

impl PlayerRuntime {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            players: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn ensure_player(&self, uid: u32) -> Result<()> {
        if self.players.lock().await.contains_key(&uid) {
            return Ok(());
        }

        let info = self.db.load_or_create_player(uid).await?;
        self.players
            .lock()
            .await
            .entry(uid)
            .or_insert(PlayerSession { info, dirty: false });
        Ok(())
    }

    pub async fn handle_packet(&self, uid: u32, packet: Packet) -> Result<Vec<Packet>> {
        self.ensure_player(uid).await?;

        let (out, save) = {
            let mut players = self.players.lock().await;
            let session = players
                .get_mut(&uid)
                .ok_or_else(|| anyhow!("player {uid} is not loaded"))?;
            let out = handle_player_packet(uid, &mut session.info, &mut session.dirty, packet);
            let save = session.dirty.then(|| session.info.clone());
            session.dirty = false;
            (out, save)
        };

        if let Some(info) = save {
            self.db.save_player(uid, &info).await?;
        }

        Ok(out)
    }
}

fn handle_player_packet(
    uid: u32,
    player: &mut PlayerInfo,
    dirty: &mut bool,
    packet: Packet,
) -> Vec<Packet> {
    let mut out = Vec::new();

    match packet.cmd_id {
        Cs10022::CMD_ID => {
            if let Some(req) = packet.decode::<Cs10022>() {
                push(
                    &mut out,
                    Sc10023 {
                        db_load: None,
                        result: 0,
                        user_id: player.uid(),
                        server_ticket: req.server_ticket,
                        server_load: None,
                    },
                    packet.id,
                );
            }
        }
        Cs10024::CMD_ID => {
            if let Some(req) = packet.decode::<Cs10024>() {
                player.init(uid, req.nick_name, req.ship_id);
                *dirty = true;
                push(
                    &mut out,
                    Sc10025 {
                        result: 0,
                        user_id: player.uid(),
                    },
                    packet.id,
                );
            }
        }
        Cs11001::CMD_ID => load_player_data(player, packet.id, &mut out),
        Cs10100::CMD_ID => {
            if packet.decode::<Cs10100>().is_some() {
                push(&mut out, Sc10101 { state: 0 }, packet.id);
            }
        }
        Cs11017::CMD_ID => {
            if let Some(req) = packet.decode::<Cs11017>() {
                player.story_list.push(req.story_id);
                *dirty = true;
                push(&mut out, Sc11018::default(), packet.id);
            }
        }
        Cs11603::CMD_ID => {
            if packet.decode::<Cs11603>().is_some() {
                push(&mut out, Sc11604::default(), packet.id);
            }
        }
        Cs11011::CMD_ID => {
            if let Some(req) = packet.decode::<Cs11011>() {
                player.character = req.character;
                *dirty = true;
                push(&mut out, Sc11012::default(), packet.id);
            }
        }
        Cs12202::CMD_ID => {
            if let Some(req) = packet.decode::<Cs12202>() {
                if let Some(ship) = player.ships.iter_mut().find(|ship| ship.id == req.ship_id) {
                    ship.skin_id = req.skin_id;
                }
                *dirty = true;
                push(&mut out, Sc12203::default(), packet.id);
            }
        }
        Cs12299::CMD_ID => {
            let _ = packet.decode::<Cs12299>();
        }
        Cs12406::CMD_ID => {
            if packet.decode::<Cs12406>().is_some() {
                push(&mut out, Sc12407::default(), packet.id);
            }
        }
        Cs11701::CMD_ID => {
            if let Some(req) = packet.decode::<Cs11701>() {
                push(
                    &mut out,
                    Sc11702 {
                        result: 0,
                        data: Some(InsMessage {
                            id: req.id,
                            is_good: (req.cmd == 3) as u32,
                            is_read: (req.cmd == 5) as u32,
                            ..Default::default()
                        }),
                    },
                    packet.id,
                );
            }
        }
        Cs63317::CMD_ID => {
            if packet.decode::<Cs63317>().is_some() {
                push(&mut out, Sc63318::default(), packet.id);
            }
        }
        Cs11710::CMD_ID => {
            if packet.decode::<Cs11710>().is_some() {
                push(&mut out, Sc11711::default(), packet.id);
            }
        }
        Cs11705::CMD_ID => {
            if packet.decode::<Cs11705>().is_some() {
                push(&mut out, Sc11706::default(), packet.id);
            }
        }
        Cs11722::CMD_ID => {
            if packet.decode::<Cs11722>().is_some() {
                push(&mut out, Sc11723::default(), packet.id);
            }
        }
        Cs11751::CMD_ID => {
            if packet.decode::<Cs11751>().is_some() {
                push(&mut out, reflux_data(), packet.id);
            }
        }
        Cs26101::CMD_ID => {
            if packet.decode::<Cs26101>().is_some() {
                push(&mut out, Sc26102::default(), packet.id);
            }
        }
        Cs24020::CMD_ID => {
            if packet.decode::<Cs24020>().is_some() {
                push(
                    &mut out,
                    Sc24021 {
                        awards: (10022..=10024)
                            .map(|key| Kvdata { key, value: 0 })
                            .collect(),
                        ..Default::default()
                    },
                    packet.id,
                );
            }
        }
        Cs34501::CMD_ID => {
            if packet.decode::<Cs34501>().is_some() {
                push(&mut out, Sc34502::default(), packet.id);
            }
        }
        Cs18001::CMD_ID => {
            if packet.decode::<Cs18001>().is_some() {
                push(
                    &mut out,
                    Sc18002 {
                        rank: 1,
                        ..Default::default()
                    },
                    packet.id,
                );
            }
        }
        Cs19009::CMD_ID => {
            if packet.decode::<Cs19009>().is_some() {
                push(&mut out, Sc19010::default(), packet.id);
            }
        }
        Cs60037::CMD_ID => {
            if packet.decode::<Cs60037>().is_some() {
                push(
                    &mut out,
                    Sc60000 {
                        guild: GuildInfo {
                            member: vec![],
                            base: GuildBaseInfo {
                                faction: 1,
                                level: 1,
                                policy: 1,
                                ..Default::default()
                            },
                            log: vec![],
                            guild_ex: Default::default(),
                        },
                    },
                    packet.id,
                );
            }
        }
        Cs62100::CMD_ID => {
            if packet.decode::<Cs62100>().is_some() {
                push(&mut out, Sc62101::default(), packet.id);
            }
        }
        Cs60102::CMD_ID => {
            if packet.decode::<Cs60102>().is_some() {
                push(
                    &mut out,
                    Sc60103 {
                        user_info: UserGuildInfo {
                            donate_tasks: vec![1, 13, 2],
                            ..Default::default()
                        },
                    },
                    packet.id,
                );
            }
        }
        Cs27000::CMD_ID => {
            if packet.decode::<Cs27000>().is_some() {
                push(
                    &mut out,
                    Sc27001 {
                        child: ChildInfo {
                            attrs: (101..=104)
                                .chain(201..=203)
                                .chain(301..=306)
                                .map(|id| ChildAttr { id, val: 0 })
                                .collect(),
                            mood: 50,
                            new_game_plus_count: 1,
                            money: 20,
                            tasks: (101..=103)
                                .map(|id| ChildTask { id, progress: 0 })
                                .collect(),
                            can_trigger_home_event: 1,
                            cur_time: ChildTime {
                                day: 0,
                                week: 1,
                                month: 1,
                            },
                            favor: ChildFavor { exp: 30, lv: 1 },
                            tid: 1,
                            ..Default::default()
                        },
                        result: 0,
                    },
                    packet.id,
                );
            }
        }
        Cs29001::CMD_ID => {
            if let Some(req) = packet.decode::<Cs29001>() {
                push(
                    &mut out,
                    Sc29002 {
                        permanent: Tbpermanent {
                            ng_plus_count: 1,
                            ..Default::default()
                        },
                        result: 0,
                        tb: new_educate_tb(req.id),
                    },
                    packet.id,
                );
            }
        }
        Cs27010::CMD_ID => {
            if packet.decode::<Cs27010>().is_some() {
                push(&mut out, Sc27011::default(), packet.id);
            }
        }
        Cs25026::CMD_ID => {
            if packet.decode::<Cs25026>().is_some() {
                push(
                    &mut out,
                    Sc25027 {
                        slots: vec![Commanderhomeslot {
                            id: 1,
                            op_flag: 7,
                            style: 1,
                            ..Default::default()
                        }],
                        level: 1,
                        ..Default::default()
                    },
                    packet.id,
                );
            }
        }
        Cs16104::CMD_ID => {
            if packet.decode::<Cs16104>().is_some() {
                push(&mut out, Sc16105::default(), packet.id);
            }
        }
        Cs15008::CMD_ID => {
            push(&mut out, Sc15009::default(), packet.id);
        }
        Cs15300::CMD_ID => {
            let _ = packet.decode::<Cs15300>();
        }
        Cs13505::CMD_ID => {
            if packet.decode::<Cs13505>().is_some() {
                push(&mut out, Sc13506::default(), packet.id);
            }
        }
        Cs20007::CMD_ID => {
            if packet.decode::<Cs20007>().is_some() {
                push(&mut out, Sc20008::default(), packet.id);
            }
        }
        Cs34001::CMD_ID => {
            if let Some(req) = packet.decode::<Cs34001>() {
                push(
                    &mut out,
                    Sc34002 {
                        meta_ship_list: req
                            .group_id
                            .iter()
                            .map(|id| MetaShipInfo {
                                group_id: *id,
                                ..Default::default()
                            })
                            .collect(),
                    },
                    packet.id,
                );
            }
        }
        Cs50102::CMD_ID => {
            if let Some(req) = packet.decode::<Cs50102>() {
                handle_chat(player, dirty, req.content, &mut out);
            }
        }
        _ => {
            tracing::warn!(
                proto = request_proto_name(packet.cmd_id),
                cmd_id = packet.cmd_id,
                packet_id = packet.id,
                wire_len = packet.raw_len(),
                length = packet.length,
                flag = packet.flag,
                payload_len = packet.data.len(),
                raw = %packet.raw_hex_prefix(48),
                "unhandled game packet"
            );
        }
    }

    out
}

fn load_player_data(player: &mut PlayerInfo, id: u16, out: &mut Vec<Packet>) {
    push(
        out,
        Sc11000 {
            timestamp: time::now_timestamp_s() as u32,
            monday_0oclock_timestamp: MONDAY_0_TIMESTAMP,
        },
        id,
    );
    push(out, player.notify_player_data(), id);
    push(out, player.notify_player_buff(), id);
    push(out, reflux_data(), id);
    push(out, Sc21536::default(), id);
    push(out, player.notify_naval_academy(), id);
    push(out, Sc26120::default(), id);
    push(out, player.notify_commander_data(), id);
    push(out, player.notify_statistics(), id);
    push(out, Sc22300::default(), id);
    push(out, Sc12024::default(), id);
    push_ship_data(out, player, id);
    push(out, player.notify_fleet_data(), id);
    push(out, player.notify_player_ship_skins_data(), id);
    push(out, Sc63000::default(), id);
    push(out, Sc63100::default(), id);
    push(out, Sc64000::default(), id);
    push(out, player.notify_chapter_info(), id);
    push(out, player.notify_current_chapter(), id);
    push(
        out,
        Sc13002 {
            max_team: 2,
            collection_list: [30107, 30102, 30101, 30103, 20113]
                .iter()
                .map(|id| Collectioninfo {
                    id: *id,
                    ..Default::default()
                })
                .collect(),
        },
        id,
    );
    push(out, Sc13201::default(), id);
    push(
        out,
        Sc16200 {
            month: time::get_month_timestamp_s() as u32,
            ..Default::default()
        },
        id,
    );
    push(out, player.notify_world_data(), id);
    push(out, player.notify_equip_data(), id);
    push(out, player.notify_equip_skin_data(), id);
    push(out, player.notify_bag_data(), id);
    push(out, Sc20001::default(), id);
    push(out, Sc20101::default(), id);
    push(out, Sc20201::default(), id);
    push(out, player.notify_dorm_data(), id);
    push(
        out,
        Sc12031 {
            energy_auto_increase_time: time::now_timestamp_s() as u32,
        },
        id,
    );
    push(out, Sc28000::default(), id);
    push(out, Sc30001::default(), id);
    push(out, Sc30101::default(), id);
    push(out, Sc50000::default(), id);
    push(out, Sc11200::default(), id);
    push(out, Sc11210::default(), id);
    push(out, server_notice(), id);
    push(
        out,
        Sc11002 {
            ship_count: player.ship_count(),
            timestamp: time::now_timestamp_s() as u32,
            monday_0oclock_timestamp: MONDAY_0_TIMESTAMP,
        },
        id,
    );
}

fn push_ship_data(out: &mut Vec<Packet>, player: &mut PlayerInfo, id: u16) {
    let (first, rest) = player.notify_player_ships_data();
    push(out, first, id);
    if let Some(rest) = rest {
        for chunk in rest {
            push(
                out,
                Sc12010 {
                    ship_list: chunk.ship_list,
                },
                id,
            );
        }
    }
}

fn handle_chat(player: &mut PlayerInfo, dirty: &mut bool, content: String, out: &mut Vec<Packet>) {
    let args = content.split_whitespace().collect::<Vec<_>>();
    let response = match args.first().copied() {
        Some("ship") => {
            give_all_ships(player);
            *dirty = true;
            push_ship_data(out, player, 0);
            "Already give all ships to you~ Have Fun!"
        }
        Some("skin") => {
            give_all_skins(player);
            *dirty = true;
            push(out, player.notify_player_ship_skins_data(), 0);
            "Already give all skins to you~ Have Fun!"
        }
        Some("help") | Some(_) => "1. ship: Give all ships. 2. skin: Give all skins",
        None => "Command Error, using `help` to show all available commands.",
    };

    push(
        out,
        Sc50101 {
            content: response.to_string(),
            r#type: 0,
            player: ChatPlayerInfo {
                id: 0,
                lv: 150,
                name: "Server".to_string(),
                display: Some(Displayinfo {
                    skin: 701042,
                    icon_frame: 501,
                    chat_frame: 103,
                    icon: 701044,
                    icon_theme: 0,
                    marry_flag: 1_580_000_000,
                    transform_flag: 0,
                }),
            },
        },
        0,
    );
}

fn give_all_ships(player: &mut PlayerInfo) {
    if let Some(data) = crate::data::ship_data_template_data::DATA.get() {
        for (key, ship) in &data.0 {
            if ship.star == ship.star_max && ship.star >= 5 {
                if let Ok(id) = key.parse() {
                    player.add_ship(id);
                }
            }
        }
    }
}

fn give_all_skins(player: &mut PlayerInfo) {
    let ship_ids = player
        .ships
        .iter()
        .map(|ship| ship.template_id)
        .collect::<Vec<_>>();
    for id in ship_ids {
        player.add_ship_skin(id);
    }
}

fn new_educate_tb(id: u32) -> Tbinfo {
    let (attrs, resource, benefit_id) = if id == 2 {
        (
            vec![(305, 145), (304, 0), (303, 0), (302, 0), (301, 0)],
            vec![
                (306, 5),
                (305, 1),
                (304, 50),
                (303, 0),
                (302, 50),
                (301, 50),
            ],
            10001,
        )
    } else {
        (
            vec![(201, 155), (104, 0), (103, 0), (102, 0), (101, 0)],
            vec![(4, 50), (3, 0), (2, 50), (1, 50)],
            10000,
        )
    };

    Tbinfo {
        id,
        fsm: Tbfsm {
            system_no: 0,
            current_node: 0,
            cache: vec![Tbfsmcache::default()],
            priority_fsm: vec![],
            tarot_selects: vec![],
        },
        round: Tbround {
            round: 1,
            in_temp: 0,
            temp_round: 0,
        },
        res: Tbres {
            attrs: attrs
                .into_iter()
                .map(|(key, value)| Kvdata { key, value })
                .collect(),
            resource: resource
                .into_iter()
                .map(|(key, value)| Kvdata { key, value })
                .collect(),
        },
        talent: Tbtalent::default(),
        plan: Tbplan::default(),
        site: Tbsite::default(),
        evaluations: vec![],
        name: String::new(),
        favor_lv: 0,
        benefit: Tbbenefit {
            actives: vec![Tbbf {
                id: benefit_id,
                round: 1,
                is_pending: 0,
            }],
        },
        difficulty: 0,
        eval_fail: 0,
        display: Tbdisplay::default(),
    }
}

fn server_notice() -> Sc11300 {
    Sc11300 {
        notice_list: vec![Noticeinfo {
            tag_type: 1,
            id: 6,
            version: "1".to_string(),
            icon: 2,
            content: NOTICE.to_string(),
            title: "Welcome to Cheshire Lane | 欢迎使用 Cheshire Lane".to_string(),
            track: String::new(),
            priority: 80,
            need_level: 0,
            btn_title: "Welcome".to_string(),
            title_image: format!(
                "http://{}:{}/static/cheshire-banner.png",
                CONFIG.sdk_ip,
                CONFIG.sdk_http_addr.port()
            ),
            time_desc: "2/1/2025".to_string(),
        }],
    }
}

fn reflux_data() -> Sc11752 {
    Sc11752 {
        active: 0,
        return_lv: Some(0),
        return_time: Some(0),
        ship_number: Some(0),
        last_offline_time: Some(0),
        pt: Some(0),
        sign_cnt: Some(0),
        sign_last_time: Some(0),
        pt_stage: Some(0),
    }
}

fn push<T: proto::CheshireMessage>(out: &mut Vec<Packet>, message: T, id: u16) {
    out.push(Packet::encode(&message, id));
}

pub(crate) fn request_proto_name(cmd_id: u16) -> &'static str {
    match cmd_id {
        Cs10022::CMD_ID => "p10.Cs10022",
        Cs10024::CMD_ID => "p10.Cs10024",
        Cs10100::CMD_ID => "p10.Cs10100",
        Cs11001::CMD_ID => "p11.Cs11001",
        Cs11011::CMD_ID => "p11.Cs11011",
        Cs11017::CMD_ID => "p11.Cs11017",
        Cs11603::CMD_ID => "p11.Cs11603",
        Cs11701::CMD_ID => "p11.Cs11701",
        Cs11705::CMD_ID => "p11.Cs11705",
        Cs11710::CMD_ID => "p11.Cs11710",
        Cs11722::CMD_ID => "p11.Cs11722",
        Cs11751::CMD_ID => "p11.Cs11751",
        Cs12202::CMD_ID => "p12.Cs12202",
        Cs12299::CMD_ID => "p12.Cs12299",
        Cs12406::CMD_ID => "p12.Cs12406",
        Cs13505::CMD_ID => "p13.Cs13505",
        Cs15008::CMD_ID => "p15.Cs15008",
        Cs15300::CMD_ID => "p15.Cs15300",
        Cs16104::CMD_ID => "p16.Cs16104",
        Cs18001::CMD_ID => "p18.Cs18001",
        Cs19009::CMD_ID => "p19.Cs19009",
        Cs20007::CMD_ID => "p20.Cs20007",
        Cs24020::CMD_ID => "p24.Cs24020",
        Cs25026::CMD_ID => "p25.Cs25026",
        Cs26101::CMD_ID => "p26.Cs26101",
        Cs27000::CMD_ID => "p27.Cs27000",
        Cs27010::CMD_ID => "p27.Cs27010",
        Cs29001::CMD_ID => "p29.Cs29001",
        Cs34001::CMD_ID => "p34.Cs34001",
        Cs34501::CMD_ID => "p34.Cs34501",
        Cs50102::CMD_ID => "p50.Cs50102",
        Cs60037::CMD_ID => "p60.Cs60037",
        Cs60102::CMD_ID => "p60.Cs60102",
        Cs62100::CMD_ID => "p62.Cs62100",
        Cs63317::CMD_ID => "p63.Cs63317",
        _ => "unknown",
    }
}

const NOTICE: &str = r#"
<size=35>Disclaimer</size>
        ※This project is intended for educational and research purposes only. Do not use it for any illegal or inappropriate activities.
        ※When using this project, ensure compliance with local laws and regulations and take full responsibility for your actions.
        ※The author is not responsible for any misuse, illegal use, or consequences arising from it.
        ※This project is open-source and free. If you have paid to use this software, please request a refund immediately.

<size=35>免责声明</size>
        ※本项目仅用于教育和研究目的，请勿将其用于任何非法或不当用途。
        ※使用本项目时，请确保遵守所在地区的法律法规，并承担相应责任。
        ※作者不对任何滥用、违法使用或由此产生的后果负责。
        ※本项目为开源免费项目，如您因使用本软件而支付费用，请立即申请退款。
"#;
