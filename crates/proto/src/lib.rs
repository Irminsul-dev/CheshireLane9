#![allow(unused, warnings)]

pub mod common;
pub mod guild;
pub mod p10;
pub mod p11;
pub mod p12;
pub mod p13;
pub mod p14;
pub mod p15;
pub mod p16;
pub mod p17;
pub mod p18;
pub mod p19;
pub mod p20;
pub mod p21;
pub mod p22;
pub mod p23;
pub mod p24;
pub mod p25;
pub mod p26;
pub mod p27;
pub mod p28;
pub mod p29;
pub mod p30;
pub mod p33;
pub mod p34;
pub mod p40;
pub mod p50;
pub mod p60;
pub mod p61;
pub mod p62;
pub mod p63;
pub mod p64;
pub mod p70;

pub use prost::Message as ProtoMsg;

pub trait CmdID {
    const CMD_ID: u16;

    fn get_cmd_id(&self) -> u16 {
        Self::CMD_ID
    }
}

pub trait CheshireMessage: ProtoMsg + CmdID {}
impl<T: ProtoMsg + CmdID> CheshireMessage for T {}
