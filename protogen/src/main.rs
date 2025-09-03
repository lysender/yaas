use yaas::buffed::actor::CredentialsBuf;
use yaas::buffed::dto::{ChangeCurrentPasswordBuf, SetupBodyBuf};

fn main() {
    write_credentials();
    write_setup_payload();
    write_change_password_payload();

    println!("Done");
}

fn write_change_password_payload() {
    let body = ChangeCurrentPasswordBuf {
        current_password: "password123".to_string(),
        new_password: "password".to_string(),
    };

    let filename = "buffs/change_password.buf";
    let bytes = prost::Message::encode_to_vec(&body);

    std::fs::write(filename, &bytes).expect("Unable to write file");
}

fn write_setup_payload() {
    let body = SetupBodyBuf {
        setup_key: "sup_01990815592d74f3bc09eb544a6e65ce".to_string(),
        email: "shanks@lysender.com".to_string(),
        password: "password".to_string(),
    };

    let filename = "buffs/setup.buf";
    let bytes = prost::Message::encode_to_vec(&body);

    std::fs::write(filename, &bytes).expect("Unable to write file");
}

fn write_credentials() {
    let credentials = CredentialsBuf {
        email: "shanks@lysender.com".to_string(),
        password: "password".to_string(),
    };

    let filename = "buffs/credentials.buf";
    let bytes = prost::Message::encode_to_vec(&credentials);

    std::fs::write(filename, &bytes).expect("Unable to write file");
}
