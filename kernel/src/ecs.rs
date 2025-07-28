use crate::actors::ActorHandle;
use bevy::prelude::*;

pub fn tag_actor<M, B>(mut commands: Commands, handle: ActorHandle<M>, bundle: B) -> Entity
where
    B: Bundle,
    M: Send + 'static,
{
    let entity = commands.spawn(handle).id();
    commands.entity(entity).insert(bundle);
    entity
}

#[derive(Component)]
pub struct TaggedActorHandle<Mes> {
    handle: ActorHandle<Mes>,
    entity: Entity,
}
