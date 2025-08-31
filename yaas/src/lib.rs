pub mod buffed {
    include!(concat!(env!("OUT_DIR"), "/yaas.rs"));
}

pub mod actor;
pub mod dto;
pub mod pagination;
pub mod role;
pub mod utils;
pub mod validators;
