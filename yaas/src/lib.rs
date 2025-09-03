pub mod buffed {
    pub mod actor {
        include!(concat!(env!("OUT_DIR"), "/buffed.actor.rs"));
    }

    pub mod dto {
        include!(concat!(env!("OUT_DIR"), "/buffed.dto.rs"));
    }

    pub mod pagination {
        include!(concat!(env!("OUT_DIR"), "/buffed.pagination.rs"));
    }

    pub mod role {
        include!(concat!(env!("OUT_DIR"), "/buffed.role.rs"));
    }
}

pub mod actor;
pub mod dto;
pub mod pagination;
pub mod role;
pub mod utils;
pub mod validators;
