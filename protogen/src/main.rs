use yaas::buffed::actor::CredentialsBuf;
use yaas::buffed::dto::SetupBodyBuf;

fn main() {
    write_credentials();
    write_setup_payload();

    println!("Done");
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
