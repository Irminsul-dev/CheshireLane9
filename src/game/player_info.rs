use proto::common::{
    Appreciationinfo, Commanderinfo, Displayinfo, EquipskinInfo, Idtimeinfo, Kvdata, Shipinfo,
    Shipskill,
};
use proto::p11::{Benefitbuff, Resource, Sc11003, Sc11015};
use proto::p12::{Groupinfo, Sc12001, Sc12010, Sc12101, Sc12201};
use proto::p13::{Chapterinfo, Currentchapterinfo, Sc13000, Sc13001};
use proto::p14::{Sc14001, Sc14101};
use proto::p15::{Iteminfo, Sc15001};
use proto::p17::{Sc17001, ShipStatisticsInfo};
use proto::p19::Sc19001;
use proto::p22::Sc22001;
use proto::p25::{Commanderboxinfo, Presetfleet, Sc25001};
use proto::p33::Sc33114;
use serde::{Deserialize, Serialize};
use std::cmp::PartialEq;

#[derive(Serialize, Deserialize, Default, Clone, Eq, PartialEq)]
pub enum ResourceType {
    #[default]
    Gold = 1,
    Oil = 2,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct ResourceField {
    pub r#type: ResourceType,
    pub level: u32,
    pub last_harvest_time: u32,
    pub upgrade_time: u32,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct PlayerInfo {
    // Player Data
    pub uid: u32,
    pub nick_name: String,
    pub level: u32,
    pub display: Displayinfo,
    pub register_time: u32,
    pub appreciation: Appreciationinfo,
    pub chat_room_id: u32,
    pub commander_bag_max: u32,
    pub equip_bag_max: u32,
    pub guide_index: u32,
    pub mail_storeroom_lv: u32,
    pub max_rank: u32,
    pub pvp_attack_count: u32,
    pub pvp_win_count: u32,
    pub rank: u32,
    pub rmb: u32,
    pub ship_bag_max: u32,
    pub win_count: u32,
    pub character: Vec<Kvdata>,
    pub story_list: Vec<u32>,
    pub adv: String,
    // Player Ships Data
    pub ships: Vec<Shipinfo>,
    // Player Ship Skins Data
    pub ship_skins: Vec<Idtimeinfo>,
    // Resource list
    pub resource_list: Vec<Resource>,
    pub resource_fields: Vec<ResourceField>,
    // Bag Data
    pub bag: Vec<Iteminfo>,
    // Equip Data
    pub sp_weapon_bag_size: u32,
    // Dorm Data
    pub dorm_level: u32,
    pub dorm_floor: u32,
    pub dorm_exp_pos: u32,
    // Naval Academy
    pub class_level: u32,
    pub skill_class_num: u32,
    // Fleet
    pub fleet: Vec<Groupinfo>,
    // Chapter Info
    pub chapter_infos: Vec<Chapterinfo>,
    pub current_chapter: Option<Currentchapterinfo>,
    // World
    pub is_world_open: bool,
    // Buff
    pub buff_list: Vec<Benefitbuff>,
    // Commander
    pub commanders: Vec<Commanderinfo>,
    pub presets: Vec<Presetfleet>,
    pub commander_box: Vec<Commanderboxinfo>,
}

impl PlayerInfo {
    pub fn uid(&self) -> u32 {
        self.uid
    }

    pub fn init(&mut self, uid: u32, nick_name: String, ship_id: u32) {
        // Player Data
        self.uid = uid;
        self.nick_name = nick_name;
        // default value for new account is `1`
        self.level = 100;
        self.register_time = crate::time::now_timestamp_s() as u32;
        self.display = Displayinfo {
            icon: ship_id,
            ..Default::default()
        };
        self.chat_room_id = 1;
        self.commander_bag_max = 100;
        self.equip_bag_max = 600;
        // default value for new account is `1`
        // which means the player has not completed the novice mission
        // here set to `29` = complete the novice mission
        self.guide_index = 29;
        self.mail_storeroom_lv = 1;
        self.rank = 1;
        self.ship_bag_max = 1000;
        // default value for new player is empty
        self.story_list = vec![
            // novice mission
            2400002, 2400001, 90004, 2, 1, 90012, // others
            30001, 11561, 30002, 90006, 90023, 90040, 30003, 30004, 6351, 90005, 90007, 90008,
            90009, 90030, 2200001,
        ];
        self.adv = "<color=#FFB6C1>CheshireLane</color>".to_string();
        // Ships Data
        self.add_ship(ship_id);
        self.character = vec![Kvdata {
            key: self.ships.last().map_or(1, |ship| ship.id),
            value: 0,
        }];
        self.add_ship(106011);
        // Resource
        self.add_resource(1, 3000); // Gold
        self.add_resource(2, 500); // Oil
        self.add_resource(5, 5);
        self.add_resource(7, 5);
        self.add_resource(8, 4000);
        self.resource_fields.clear();
        self.resource_fields.push(ResourceField {
            r#type: ResourceType::Gold,
            level: 1,
            last_harvest_time: crate::time::now_timestamp_s() as u32,
            upgrade_time: crate::time::now_timestamp_s() as u32,
        });
        self.resource_fields.push(ResourceField {
            r#type: ResourceType::Oil,
            level: 1,
            last_harvest_time: crate::time::now_timestamp_s() as u32,
            upgrade_time: crate::time::now_timestamp_s() as u32,
        });
        // Equip Data
        self.sp_weapon_bag_size = 150;
        // Dorm Data
        self.dorm_level = 1;
        self.dorm_floor = 1;
        self.dorm_exp_pos = 2;
        // Naval Academy
        self.class_level = 1;
        self.skill_class_num = 2;
        // World
        self.is_world_open = true;
        // Chapter
        self.chapter_infos.clear();
        for chapter_id in crate::data::chapter_template_data::DATA
            .get()
            .unwrap()
            .0
            .keys()
        {
            if let Ok(chapter_id) = chapter_id.parse::<u32>() {
                self.chapter_infos.push(Chapterinfo {
                    take_box_count: 99,
                    progress: 100,
                    today_defeat_count: 0,
                    pass_count: 99,
                    kill_enemy_count: 99,
                    id: chapter_id,
                    kill_boss_count: 99,
                    defeat_count: 99,
                });
            }
        }
    }

    pub fn add_resource(&mut self, ty: u32, num: u32) {
        self.resource_list.push(Resource { r#type: ty, num });
    }

    pub fn ship_count(&self) -> u32 {
        self.ships.len() as u32
    }

    pub fn add_ship(&mut self, ship_id: u32) {
        let (Some(ship_data), Some(ship_statistics_data), Some(ship_skin_data)) = (
            crate::data::ship_data_template_data::DATA.get(),
            crate::data::ship_data_statistics_data::DATA.get(),
            crate::data::ship_skin_template_data::DATA.get(),
        ) else {
            return;
        };

        let Some(ship_template) = ship_data.0.get(&ship_id.to_string()) else {
            return;
        };
        let Some(ship_statistics) = ship_statistics_data.0.get(&ship_template.id.to_string())
        else {
            return;
        };
        if !ship_skin_data
            .0
            .contains_key(&ship_statistics.skin_id.to_string())
        {
            return;
        }

        let id = self.ships.iter().map(|ship| ship.id).max().unwrap_or(0) + 1;
        self.ships.push(Shipinfo {
            id,
            template_id: ship_template.id,
            level: 1,
            energy: ship_template.energy,
            skill_id_list: ship_template
                .buff_list
                .iter()
                .map(|id| Shipskill {
                    skill_id: *id,
                    skill_lv: 1,
                    ..Default::default()
                })
                .collect(),
            // default intimacy value is 5000
            intimacy: 10000,
            equip_info_list: vec![
                EquipskinInfo {
                    id: ship_template.equip_id_1,
                    ..Default::default()
                },
                EquipskinInfo {
                    id: ship_template.equip_id_2,
                    ..Default::default()
                },
                EquipskinInfo {
                    id: ship_template.equip_id_3,
                    ..Default::default()
                },
                EquipskinInfo {
                    ..Default::default()
                },
                EquipskinInfo {
                    ..Default::default()
                },
            ],
            propose: 1,
            ..Default::default()
        });
    }

    pub fn add_ship_skin(&mut self, ship_id: u32) {
        if let (Some(ship_data), Some(ship_skin_data)) = (
            crate::data::ship_data_template_data::DATA.get(),
            crate::data::ship_skin_template_data::DATA.get(),
        ) {
            let group_type = if let Some(ship_template) = ship_data.0.get(&ship_id.to_string()) {
                ship_template.group_type
            } else {
                return;
            };
            let skins = ship_skin_data
                .0
                .iter()
                .filter_map(|(_, skin)| {
                    if skin.ship_group == group_type {
                        Some(skin)
                    } else {
                        None
                    }
                })
                .map(|skin| Idtimeinfo {
                    id: skin.id,
                    ..Default::default()
                })
                .collect::<Vec<Idtimeinfo>>();

            self.ship_skins.extend(&skins);
        }
    }

    pub fn notify_player_data(&self) -> Sc11003 {
        Sc11003 {
            id: self.uid,
            name: self.nick_name.clone(),
            level: self.level,
            display: Some(self.display),
            register_time: self.register_time,
            ship_count: self.ship_count(),
            resource_list: self.resource_list.clone(),
            appreciation: self.appreciation.clone(),
            chat_room_id: self.chat_room_id,
            commander_bag_max: self.commander_bag_max,
            equip_bag_max: self.equip_bag_max,
            guide_index: self.guide_index,
            new_guide_index: self.guide_index,
            mail_storeroom_lv: self.mail_storeroom_lv,
            marry_ship: self.ships.iter().filter(|ship| ship.propose == 1).count() as u32,
            max_rank: self.max_rank,
            pvp_attack_count: self.pvp_attack_count,
            pvp_win_count: self.pvp_win_count,
            rank: self.rank,
            rmb: self.rmb,
            ship_bag_max: self.ship_bag_max,
            win_count: self.win_count,
            character: self.character.clone(),
            story_list: self.story_list.clone(),
            adv: self.adv.clone(),
            ..Default::default()
        }
    }

    pub fn notify_player_ships_data(&self) -> (Sc12001, Option<Vec<Sc12010>>) {
        let mut chunks = self.ships.chunks(100);

        let first_chunk = chunks.next().unwrap_or(&[]);
        let sc12001 = Sc12001 {
            shiplist: first_chunk.to_vec(),
        };

        let sc12010s: Vec<Sc12010> = chunks
            .map(|chunk| Sc12010 {
                ship_list: chunk.to_vec(),
            })
            .collect();
        let sc12010s = if sc12010s.is_empty() {
            None
        } else {
            Some(sc12010s)
        };

        (sc12001, sc12010s)
    }

    pub fn notify_player_ship_skins_data(&self) -> Sc12201 {
        Sc12201 {
            skin_list: self.ship_skins.clone(),
            ..Default::default()
        }
    }

    pub fn notify_statistics(&self) -> Sc17001 {
        let mut ships = self.ships.clone();
        ships.sort_by_key(|info| info.template_id);
        ships.dedup_by_key(|info| info.template_id);
        ships.sort_by_key(|info| info.level);

        if let Some(ship_data) = crate::data::ship_data_template_data::DATA.get() {
            Sc17001 {
                ship_info_list: ships
                    .iter()
                    .filter_map(|info| {
                        if let Some(template) = ship_data.0.get(&info.template_id.to_string()) {
                            Some(ShipStatisticsInfo {
                                id: template.group_type,
                                star: template.star,
                                lv_max: info.level,
                                intimacy_max: info.intimacy,
                                ..Default::default()
                            })
                        } else {
                            None
                        }
                    })
                    .collect(),
                ..Default::default()
            }
        } else {
            Sc17001::default()
        }
    }

    pub fn notify_bag_data(&self) -> Sc15001 {
        Sc15001 {
            item_list: self.bag.clone(),
            ..Default::default()
        }
    }

    pub fn notify_equip_data(&self) -> Sc14001 {
        Sc14001 {
            spweapon_bag_size: self.sp_weapon_bag_size,
            ..Default::default()
        }
    }

    pub fn notify_equip_skin_data(&self) -> Sc14101 {
        Sc14101::default()
    }

    pub fn notify_dorm_data(&self) -> Sc19001 {
        Sc19001 {
            exp_pos: self.dorm_exp_pos,
            lv: self.dorm_level,
            floor_num: self.dorm_floor,
            load_time: crate::time::now_timestamp_s() as u32,
            ..Default::default()
        }
    }

    pub fn notify_naval_academy(&self) -> Sc22001 {
        Sc22001 {
            class_lv: self.class_level,
            class: Default::default(),
            skill_class_num: self.skill_class_num,
            oil_well_level: self
                .resource_fields
                .iter()
                .find(|res| res.r#type == ResourceType::Oil)
                .map_or(0, |res| res.level),
            gold_well_level: self
                .resource_fields
                .iter()
                .find(|res| res.r#type == ResourceType::Gold)
                .map_or(0, |res| res.level),
            ..Default::default()
        }
    }

    pub fn notify_fleet_data(&self) -> Sc12101 {
        Sc12101 {
            group_list: self.fleet.clone(),
        }
    }

    pub fn notify_chapter_info(&self) -> Sc13001 {
        Sc13001 {
            react_chapter: None,
            chapter_list: self.chapter_infos.clone(),
            ..Default::default()
        }
    }

    pub fn notify_current_chapter(&self) -> Sc13000 {
        Sc13000 {
            current_chapter: self.current_chapter.clone(),
            daily_repair_count: 0,
        }
    }

    pub fn notify_world_data(&self) -> Sc33114 {
        Sc33114 {
            is_world_open: if self.is_world_open { 1 } else { 0 },
            ..Default::default()
        }
    }

    pub fn notify_player_buff(&self) -> Sc11015 {
        Sc11015 {
            buff_list: self.buff_list.clone(),
        }
    }

    pub fn notify_commander_data(&self) -> Sc25001 {
        Sc25001 {
            r#box: self.commander_box.clone(),
            usage_count: self.presets.len() as u32, // ?
            commanders: self.commanders.clone(),
            presets: self.presets.clone(),
        }
    }
}
