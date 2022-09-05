//! Jumper is platform that pushes actors (players or bots) up.

use crate::{game_ref, Bot, GameConstructor, Player};
use fyrox::{
    core::{
        algebra::Vector3,
        inspect::prelude::*,
        reflect::Reflect,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
    },
    impl_component_provider,
    scene::{collider::Collider, node::TypeUuidProvider, rigidbody::RigidBody},
    script::{ScriptContext, ScriptTrait},
};
use std::collections::HashSet;

#[derive(Clone, Default, Debug, Visit, Inspect, Reflect)]
pub struct Jumper {
    push_force: f32,
}

impl_component_provider!(Jumper);

impl TypeUuidProvider for Jumper {
    fn type_uuid() -> Uuid {
        uuid!("be8a29af-c10a-4518-a78b-955c8f48a8cd")
    }
}

impl ScriptTrait for Jumper {
    fn on_update(&mut self, context: ScriptContext) {
        let game_ref = game_ref(context.plugin);
        if let Some(collider) = context.scene.graph[context.handle].cast::<Collider>() {
            let mut contacted_colliders = HashSet::new();

            for contact in collider.contacts(&context.scene.graph.physics) {
                for actor in game_ref.actors.iter() {
                    let actor_script = context.scene.graph[*actor].script();

                    if let Some(actor_collider) = actor_script
                        .and_then(|s| s.query_component_ref::<Player>().map(|p| p.collider))
                        .or_else(|| {
                            actor_script
                                .and_then(|s| s.query_component_ref::<Bot>().map(|b| b.collider))
                        })
                    {
                        if contact.collider1 == actor_collider
                            || contact.collider2 == actor_collider
                        {
                            contacted_colliders.insert(actor_collider);
                        }
                    }
                }
            }

            for collider in contacted_colliders {
                let parent = context.scene.graph[collider].parent();
                if let Some(rigid_body) = context
                    .scene
                    .graph
                    .try_get_mut(parent)
                    .and_then(|p| p.cast_mut::<RigidBody>())
                {
                    let lin_vel = rigid_body.lin_vel();
                    rigid_body.set_lin_vel(Vector3::new(lin_vel.x, self.push_force, lin_vel.z));
                }
            }
        }
    }
    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn plugin_uuid(&self) -> Uuid {
        GameConstructor::type_uuid()
    }
}
