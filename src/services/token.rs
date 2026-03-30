use base64::prelude::*;
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, ensure};

use crate::dto::{ActorPayloadDto, to_roles, to_scopes};
use crate::{
    Error, Result,
    error::{
        Base64DecodeSnafu, CsrfTokenSnafu, InvalidAuthTokenSnafu, JwtClaimsParseSnafu,
        WhateverSnafu,
    },
};

#[derive(Debug, Deserialize, Serialize)]
struct Claims {
    sub: String,
    oid: String,
    orc: i32,
    roles: String,
    scope: String,
    exp: usize,
}

// Duration in seconds
const EXP_DURATION: i64 = 60 * 60 * 24 * 14; // 2 weeks

pub fn create_auth_token(actor: &ActorPayloadDto, secret: &str) -> Result<String> {
    let exp = Utc::now() + Duration::seconds(EXP_DURATION);
    let data = actor.clone();

    let roles: Vec<String> = actor.roles.iter().map(|r| r.to_string()).collect();
    let roles = roles.join(",");

    let scopes: Vec<String> = actor.scopes.iter().map(|s| s.to_string()).collect();
    let scope = scopes.join(" ");

    let claims = Claims {
        sub: data.id,
        oid: data.org_id,
        orc: data.org_count,
        roles,
        scope,
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

pub fn verify_auth_token(token: &str, secret: &str) -> Result<ActorPayloadDto> {
    let Ok(decoded) = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    ) else {
        return InvalidAuthTokenSnafu {}.fail();
    };

    ensure!(!decoded.claims.scope.is_empty(), InvalidAuthTokenSnafu {});

    let roles = decoded
        .claims
        .roles
        .split(',')
        .map(|r| r.to_string())
        .collect::<Vec<String>>();

    let roles = to_roles(&roles)?;

    let scope_list: Vec<String> = decoded
        .claims
        .scope
        .split(' ')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    let scopes = to_scopes(&scope_list)?;

    Ok(ActorPayloadDto {
        id: decoded.claims.sub,
        org_id: decoded.claims.oid,
        org_count: decoded.claims.orc,
        roles,
        scopes,
    })
}

#[derive(Deserialize, Serialize)]
struct CsrfClaims {
    sub: String,
    exp: usize,
}

pub fn create_csrf_token_svc(subject: &str, secret: &str) -> Result<String> {
    // Limit up to 1 hour only
    let exp = Utc::now() + Duration::hours(1);

    let claims = CsrfClaims {
        sub: subject.to_string(),
        exp: exp.timestamp() as usize,
    };

    let Ok(token) = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    ) else {
        return Err("Error creating JWT token".into());
    };

    Ok(token)
}

pub fn verify_csrf_token(token: &str, secret: &str) -> Result<String> {
    let Ok(decoded) = decode::<CsrfClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    ) else {
        return Err(Error::CsrfToken);
    };

    ensure!(!decoded.claims.sub.is_empty(), CsrfTokenSnafu);
    Ok(decoded.claims.sub)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthClaims {
    pub sub: String,
    pub oid: String,
    pub roles: String,
    pub scope: String,
    pub exp: usize,
}

pub fn decode_auth_token(token: &str) -> Result<AuthClaims> {
    let chunks: Vec<&str> = token.split('.').collect();
    if let Some(data_chunk) = chunks.get(1) {
        let decoded = BASE64_URL_SAFE_NO_PAD
            .decode(*data_chunk)
            .context(Base64DecodeSnafu)?;

        let claims: AuthClaims = serde_json::from_slice(&decoded).context(JwtClaimsParseSnafu)?;
        return Ok(claims);
    }

    Err("Invalid auth token.".into())
}

#[cfg(test)]
mod tests {
    use crate::{
        dto::{Role, Scope},
        utils::{IdPrefix, generate_id},
    };

    use super::*;

    #[test]
    fn test_jwt_token() {
        // Generate token
        let user_id = generate_id(IdPrefix::User);
        let org_id = generate_id(IdPrefix::Org);

        let actor = ActorPayloadDto {
            id: user_id.clone(),
            org_id: org_id.clone(),
            org_count: 1,
            roles: vec![Role::OrgAdmin],
            scopes: vec![Scope::Auth, Scope::Vault],
        };
        let token = create_auth_token(&actor, "secret").unwrap();
        println!("Token: {}", token);
        assert!(token.len() > 0);

        // Validate it back
        let actor = verify_auth_token(&token, "secret").unwrap();
        assert_eq!(actor.id, user_id);
        assert_eq!(actor.org_id, org_id);
        assert_eq!(actor.org_count, 1);
        assert_eq!(actor.scopes, vec![Scope::Auth, Scope::Vault]);
    }

    #[test]
    fn test_expired_token() {
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWJqZWN0IjoidGhvcjAxIiwic2NvcGUiOiJhdXRoIGZpbGVzIiwiZXhwIjoxNzE5MDc2MTI2fQ.ep8nXWWHS75MxoOY_yB4m0uoWgxCz1bPNvTPIgourcQ".to_string();
        let result = verify_auth_token(&token, "secret");
        assert!(result.is_err());
    }

    #[test]
    fn test_csrf_jwt_token() {
        // Generate token
        let token = create_csrf_token_svc("example", "secret").expect("Token should be generated");
        assert!(token.len() > 0);
        println!("Token: {}", token);

        // Validate it back
        let value = verify_csrf_token(&token, "secret").expect("Token should be verified");
        assert_eq!(value, "example".to_string());
    }

    #[test]
    fn test_csrf_expired_token() {
        let token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJleGFtcGxlIiwiZXhwIjoxNzIxMDk1MDIyfQ.7ddeJN3Tys_8kc8a02umkNLv42lPHIoSDaqmi-WjRhE".to_string();
        let result = verify_csrf_token(&token, "secret");
        assert!(result.is_err());
    }
}
