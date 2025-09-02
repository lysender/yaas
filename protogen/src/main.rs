use yaas::buffed::actor::CredentialsBuf;
use yaas::buffed::dto::OrgBuf;

fn main() {
    println!("Hello, world!");

    let buffed_org = OrgBuf {
        id: 2000,
        name: "Example Org".to_string(),
        status: "active".to_string(),
        owner_id: 1000,
        created_at: "2024-10-01T12:00:00Z".to_string(),
        updated_at: "2024-10-01T12:00:00Z".to_string(),
    };

    println!("Buffed Org: {:?}", buffed_org);

    let credentials = CredentialsBuf {
        email: "foo@bar.com".to_string(),
        password: "zehahaha".to_string(),
    };

    let filename = "buffs/credentials.buf";
    let bytes = prost::Message::encode_to_vec(&credentials);

    // Save to file
    std::fs::write(filename, &bytes).expect("Unable to write file");
}
