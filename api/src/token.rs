use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, ensure};

use crate::{
    Result,
    error::{InvalidAuthTokenSnafu, InvalidRolesSnafu, WhateverSnafu},
};
use yaas::{actor::ActorPayload, role::to_roles};

#[derive(Debug, Deserialize, Serialize)]
struct Claims {
    sub: String,
    oid: String,
    roles: String,
    scope: String,
    exp: usize,
}

// Duration in seconds
const EXP_DURATION: i64 = 60 * 60 * 24 * 14; // 2 weeks

pub fn create_auth_token(actor: &ActorPayload, secret: &str) -> Result<String> {
    let exp = Utc::now() + Duration::seconds(EXP_DURATION);
    let data = actor.clone();

    let roles: Vec<String> = actor.roles.iter().map(|r| r.to_string()).collect();
    let roles = roles.join(",");

    let claims = Claims {
        sub: data.id,
        oid: data.org_id,
        roles,
        scope: data.scope,
        exp: exp.timestamp() as usize,
    };

    let Ok(token) = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    ) else {
        return WhateverSnafu {
            msg: "Error creating JWT token".to_string(),
        }
        .fail();
    };

    Ok(token)
}

pub fn verify_auth_token(token: &str, secret: &str) -> Result<ActorPayload> {
    let Ok(decoded) = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    ) else {
        return InvalidAuthTokenSnafu {}.fail();
    };

    ensure!(decoded.claims.sub.len() > 0, InvalidAuthTokenSnafu {});
    ensure!(decoded.claims.scope.len() > 0, InvalidAuthTokenSnafu {});

    let roles = decoded
        .claims
        .roles
        .split(',')
        .map(|r| r.to_string())
        .collect::<Vec<String>>();

    let roles = to_roles(&roles).context(InvalidRolesSnafu)?;

    Ok(ActorPayload {
        id: decoded.claims.sub,
        org_id: decoded.claims.oid,
        roles,
        scope: decoded.claims.scope,
    })
}

#[cfg(test)]
mod tests {
    use yaas::role::Role;

    use super::*;

    #[test]
    fn test_jwt_token() {
        // Generate token
        let actor = ActorPayload {
            id: "thor01".to_string(),
            org_id: "org01".to_string(),
            roles: vec![Role::OrgAdmin],
            scope: "auth vault".to_string(),
        };
        let token = create_auth_token(&actor, "secret").unwrap();
        println!("Token: {}", token);
        assert!(token.len() > 0);

        // Validate it back
        let actor = verify_auth_token(&token, "secret").unwrap();
        assert_eq!(actor.id, "thor01".to_string());
        assert_eq!(actor.org_id, "org01".to_string());
        assert_eq!(actor.scope, "auth vault".to_string());
    }

    #[test]
    fn test_expired_token() {
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWJqZWN0IjoidGhvcjAxIiwic2NvcGUiOiJhdXRoIGZpbGVzIiwiZXhwIjoxNzE5MDc2MTI2fQ.ep8nXWWHS75MxoOY_yB4m0uoWgxCz1bPNvTPIgourcQ".to_string();
        let result = verify_auth_token(&token, "secret");
        assert!(result.is_err());
    }
}
