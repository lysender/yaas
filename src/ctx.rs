use crate::dto::{Actor, ActorDto};

#[derive(Clone)]
pub struct Ctx {
    pub actor: Actor,
}

impl Ctx {
    pub fn new(actor: Actor) -> Self {
        Ctx { actor }
    }

    pub fn actor(&self) -> Option<&ActorDto> {
        if let Some(actor) = self.actor.actor.as_ref() {
            return Some(actor);
        }
        None
    }
}
