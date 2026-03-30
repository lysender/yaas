use yaas::dto::{Actor, ActorDto};

#[derive(Clone)]
pub struct Ctx {
    pub actor: Actor,
    pub token: Option<String>,
}

impl Ctx {
    pub fn new(actor: Actor, token: Option<String>) -> Self {
        Ctx { actor, token }
    }

    pub fn token(&self) -> Option<&str> {
        if let Some(token) = self.token.as_ref() {
            return Some(token);
        }
        None
    }

    pub fn actor(&self) -> Option<&ActorDto> {
        if let Some(actor) = self.actor.actor.as_ref() {
            return Some(actor);
        }
        None
    }
}
