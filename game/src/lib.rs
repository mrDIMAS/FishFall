//! Game project.
use crate::{
    bot::Bot, camera::CameraController, cannon::Cannon, jumper::Jumper, menu::Menu,
    obstacle::RotatorObstacle, player::Player, ragdoll::link::BoneLink, ragdoll::Ragdoll,
    respawn::RespawnZone, start::StartPoint, target::Target,
};
use fyrox::{
    core::{color::Color, futures::executor::block_on, pool::Handle},
    event::Event,
    event_loop::ControlFlow,
    gui::message::UiMessage,
    plugin::{Plugin, PluginConstructor, PluginContext, PluginRegistrationContext},
    scene::{node::Node, Scene, SceneLoader},
    utils::log::Log,
};
use std::collections::HashSet;

pub mod bot;
pub mod camera;
pub mod cannon;
pub mod jumper;
pub mod marker;
pub mod menu;
pub mod obstacle;
pub mod player;
pub mod ragdoll;
pub mod respawn;
pub mod start;
pub mod target;
pub mod utils;

pub struct Game {
    menu: Menu,
    scene: Handle<Scene>,
    pub targets: HashSet<Handle<Node>>,
    pub start_points: HashSet<Handle<Node>>,
    pub actors: HashSet<Handle<Node>>,
}

pub struct GameConstructor;

impl PluginConstructor for GameConstructor {
    fn register(&self, context: PluginRegistrationContext) {
        let script_constructors = &context.serialization_context.script_constructors;
        script_constructors
            .add::<Player>("Player")
            .add::<CameraController>("Camera Controller")
            .add::<Bot>("Bot")
            .add::<Target>("Target")
            .add::<RotatorObstacle>("Rotator Obstacle")
            .add::<StartPoint>("Start Point")
            .add::<RespawnZone>("Respawn Zone")
            .add::<Cannon>("Cannon")
            .add::<Jumper>("Jumper")
            .add::<Ragdoll>("Ragdoll")
            .add::<BoneLink>("Bone Link");
    }

    fn create_instance(
        &self,
        override_scene: Handle<Scene>,
        context: PluginContext,
    ) -> Box<dyn Plugin> {
        Box::new(Game::new(override_scene, context))
    }
}

impl Game {
    fn new(override_scene: Handle<Scene>, mut context: PluginContext) -> Self {
        Log::info("Game started!".to_owned());

        let scene = if override_scene.is_some() {
            override_scene
        } else {
            let scene = block_on(
                block_on(SceneLoader::from_file(
                    "data/drake.rgs",
                    context.serialization_context.clone(),
                ))
                .unwrap()
                .finish(context.resource_manager.clone()),
            );

            context.scenes.add(scene)
        };

        if let Some(scene) = context.scenes.try_get_mut(scene) {
            scene.ambient_lighting_color = Color::opaque(150, 150, 150);

            Log::info("Scene was set successfully!".to_owned());
        }

        Self {
            menu: Menu::new(&mut context),
            targets: Default::default(),
            start_points: Default::default(),
            actors: Default::default(),
            scene,
        }
    }
}

impl Plugin for Game {
    fn on_deinit(&mut self, _context: PluginContext) {
        Log::info("Game stopped!".to_owned());
    }

    fn on_os_event(
        &mut self,
        event: &Event<()>,
        context: PluginContext,
        _control_flow: &mut ControlFlow,
    ) {
        self.menu.handle_os_event(event, context);
    }

    fn update(&mut self, context: &mut PluginContext, _control_flow: &mut ControlFlow) {
        if false {
            if let Some(scene) = context.scenes.try_get_mut(self.scene) {
                scene.drawing_context.clear_lines();

                scene.graph.physics.draw(&mut scene.drawing_context);
            }
        }
    }

    fn on_ui_message(
        &mut self,
        context: &mut PluginContext,
        message: &UiMessage,
        control_flow: &mut ControlFlow,
    ) {
        self.menu.handle_ui_message(context, message, control_flow);
    }
}

pub fn game_ref(plugins: &[Box<dyn Plugin>]) -> &Game {
    plugins.first().unwrap().cast::<Game>().unwrap()
}

pub fn game_mut(plugins: &mut [Box<dyn Plugin>]) -> &mut Game {
    plugins.first_mut().unwrap().cast_mut::<Game>().unwrap()
}
