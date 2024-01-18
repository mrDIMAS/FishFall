//! Object marker components.

use crate::{utils, Game};
use fyrox::{
    core::{algebra::Vector3, pool::Handle, reflect::prelude::*, visitor::prelude::*},
    scene::{collider::Collider, graph::Graph, node::Node, ragdoll::Ragdoll, rigidbody::RigidBody},
    script::{ScriptContext, ScriptMessageContext, ScriptMessagePayload},
};

#[derive(Debug)]
pub enum ActorMessage {
    RespawnAt(Vector3<f32>),
}

/// A marker that indicates that an object is an actor (player or bot).
#[derive(Clone, Debug, Visit, Reflect)]
#[visit(optional)]
pub struct Actor {
    #[visit(skip)]
    #[reflect(hidden)]
    pub in_air_time: f32,
    #[reflect(
        description = "Amount of time that the bot will be lying on the ground with active ragdoll."
    )]
    max_in_air_time: f32,
    #[visit(skip)]
    #[reflect(hidden)]
    pub stand_up_timer: f32,
    #[visit(skip)]
    #[reflect(hidden)]
    pub stand_up_interval: f32,
    #[reflect(description = "A handle of the ragdoll")]
    ragdoll: Handle<Node>,
    #[visit(skip)]
    #[reflect(hidden)]
    pub jump: bool,
    #[reflect(description = "Handle to actor's collider.")]
    pub collider: Handle<Node>,
    #[reflect(description = "Handle to actor's rigid body.")]
    pub rigid_body: Handle<Node>,
    #[reflect(description = "Speed of the actor.")]
    pub speed: f32,
}

impl Default for Actor {
    fn default() -> Self {
        Self {
            in_air_time: 0.0,
            max_in_air_time: 1.1,
            stand_up_timer: 0.0,
            stand_up_interval: 1.0,
            ragdoll: Default::default(),
            jump: false,
            collider: Default::default(),
            rigid_body: Default::default(),
            speed: 4.0,
        }
    }
}

impl Actor {
    fn is_ragdoll_has_ground_contact(&self, graph: &Graph) -> bool {
        let mut result = false;
        if let Some(ragdoll) = graph.try_get_of_type::<Ragdoll>(self.ragdoll) {
            ragdoll.root_limb().iterate_recursive(&mut |limb| {
                if let Some(rigid_body) = graph.try_get_of_type::<RigidBody>(limb.physical_bone) {
                    for child in rigid_body.children() {
                        if utils::has_ground_contact(*child, graph) {
                            result = true;
                            break;
                        }
                    }
                }
            });
        }
        result
    }

    pub fn has_ground_contact(&self, graph: &Graph) -> bool {
        utils::has_ground_contact(self.collider, graph) || self.is_ragdoll_has_ground_contact(graph)
    }

    pub fn set_ragdoll_enabled(&mut self, graph: &mut Graph, enabled: bool) {
        if let Some(ragdoll) = graph.try_get_mut_of_type::<Ragdoll>(self.ragdoll) {
            ragdoll.set_active(enabled);
        }
    }

    pub fn is_ragdoll_enabled(&mut self, graph: &Graph) -> bool {
        if let Some(ragdoll) = graph.try_get_of_type::<Ragdoll>(self.ragdoll) {
            ragdoll.is_active()
        } else {
            false
        }
    }

    pub fn on_message(
        &mut self,
        message: &mut dyn ScriptMessagePayload,
        ctx: &mut ScriptMessageContext,
    ) {
        let Some(message) = message.downcast_ref::<ActorMessage>() else {
            return;
        };

        match message {
            ActorMessage::RespawnAt(position) => {
                self.set_ragdoll_enabled(&mut ctx.scene.graph, false);

                if let Some(rigid_body) = ctx.scene.graph.try_get_mut(self.rigid_body) {
                    rigid_body.local_transform_mut().set_position(*position);
                }
            }
        }
    }

    pub fn for_each_rigid_body<F>(&mut self, graph: &mut Graph, mut func: F)
    where
        F: FnMut(&mut RigidBody),
    {
        let mut mbc = graph.begin_multi_borrow::<32>();
        if let Some(rigid_body) = mbc
            .try_get(self.rigid_body)
            .and_then(|n| n.query_component_mut::<RigidBody>())
        {
            func(rigid_body)
        }
        if let Some(ragdoll) = mbc
            .try_get(self.ragdoll)
            .and_then(|n| n.query_component_ref::<Ragdoll>())
        {
            ragdoll.root_limb().iterate_recursive(&mut |limb| {
                if let Some(rigid_body) = mbc
                    .try_get(limb.physical_bone)
                    .and_then(|n| n.query_component_mut::<RigidBody>())
                {
                    func(rigid_body)
                }
            });
        }
    }

    pub fn set_velocity(&mut self, velocity: Vector3<f32>, graph: &mut Graph, xz_plane_only: bool) {
        self.for_each_rigid_body(graph, &mut |rigid_body: &mut RigidBody| {
            let y_vel = rigid_body.lin_vel().y;
            rigid_body.set_lin_vel(if xz_plane_only {
                Vector3::new(velocity.x, y_vel, velocity.z)
            } else {
                velocity
            });
        });
    }

    pub fn add_force(&mut self, force: Vector3<f32>, max_speed: f32, graph: &mut Graph) {
        self.for_each_rigid_body(graph, &mut |rigid_body: &mut RigidBody| {
            if rigid_body.lin_vel().xz().norm() < max_speed {
                rigid_body.apply_force(force);
            }
        });
    }

    pub fn do_move(&mut self, velocity: Vector3<f32>, graph: &mut Graph, has_ground_contact: bool) {
        if has_ground_contact && !self.is_ragdoll_enabled(graph) {
            self.set_velocity(velocity, graph, !self.jump);
        } else {
            self.add_force(velocity.scale(2.25), self.speed, graph);
        }
    }

    fn has_serious_impact(&mut self, ctx: &mut ScriptContext) -> bool {
        if let Some(collider) = ctx.scene.graph.try_get_of_type::<Collider>(self.collider) {
            for contact in collider.contacts(&ctx.scene.graph.physics) {
                if contact.has_any_active_contact {
                    for manifold in contact.manifolds.iter() {
                        if let (Some(rb1), Some(rb2)) = (
                            ctx.scene
                                .graph
                                .try_get_of_type::<RigidBody>(manifold.rigid_body1),
                            ctx.scene
                                .graph
                                .try_get_of_type::<RigidBody>(manifold.rigid_body2),
                        ) {
                            if (rb1.lin_vel() - rb2.lin_vel()).norm() > 1.0
                                || manifold.points.iter().any(|p| p.impulse > 0.6)
                            {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    pub fn on_update(&mut self, ctx: &mut ScriptContext) {
        let game = ctx.plugins.get::<Game>();
        let has_ground_contact = self.has_ground_contact(&ctx.scene.graph);
        if has_ground_contact {
            self.in_air_time = 0.0;
            self.stand_up_timer += ctx.dt;
            if self.stand_up_timer >= self.stand_up_interval {
                self.set_ragdoll_enabled(&mut ctx.scene.graph, false);
            }
        } else {
            self.in_air_time += ctx.dt;
            self.stand_up_timer = 0.0;
            if !game.debug_settings.disable_ragdoll && self.in_air_time >= self.max_in_air_time {
                self.set_ragdoll_enabled(&mut ctx.scene.graph, true);
            }
        }
        if self.has_serious_impact(ctx) {
            self.in_air_time = 999.0;
        }
        self.jump = false;
    }
}
