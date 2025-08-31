pub mod buffed {
    pub mod yaas {
        include!(concat!(env!("OUT_DIR"), "/buffed.rs"));
    }
}

use buffed::yaas::UserDto;
