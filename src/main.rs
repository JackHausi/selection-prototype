use amethyst::{
    assets::{AssetStorage, Handle, Loader},
    core::transform::{Transform, TransformBundle},
    ecs::prelude::{
        Component, DenseVecStorage, Entities, Entity, Join, Read, ReadExpect, ReadStorage,
        System, World, WriteStorage,
    },
    input::{InputBundle, InputHandler, StringBindings},
    prelude::*,
    renderer::{
        plugins::{RenderFlat2D, RenderToWindow},
        RenderingBundle,
        types::DefaultBackend,
    },
    renderer::{
        camera::Camera,
        ImageFormat,
        sprite::SpriteRender, sprite::SpriteSheet, SpriteSheetFormat, Texture,
    },
    ui::{Anchor, Interactable, RenderUi, Selectable, Selected, UiBundle, UiTransform},
    utils::application_root_dir,
    window::ScreenDimensions,
};
use log::info;

#[derive(Debug)]
struct SomeObject {
    ordered_to: Option<(f32, f32)>,
}

impl Component for SomeObject {
    type Storage = DenseVecStorage<Self>;
}

impl SomeObject {
    fn new() -> SomeObject {
        SomeObject { ordered_to: None }
    }
}

struct SelectedSpriteRender {
    sprite_render: SpriteRender,
}

struct MarkedAsSelected {
    index: u32,
}

impl MarkedAsSelected {
    fn new(marker_entity: &Entity) -> MarkedAsSelected {
        MarkedAsSelected {
            index: marker_entity.id(),
        }
    }
}

impl Component for MarkedAsSelected {
    type Storage = DenseVecStorage<Self>;
}

struct MarkSelectedSystem;

impl<'s> System<'s> for MarkSelectedSystem {
    type SystemData = (
        Entities<'s>,
        ReadExpect<'s, SelectedSpriteRender>,
        ReadStorage<'s, Selected>,
        WriteStorage<'s, Transform>,
        WriteStorage<'s, SpriteRender>,
        WriteStorage<'s, MarkedAsSelected>,
    );

    fn run(
        &mut self,
        (
            entities,
            selected_sprite_render,
            selecteds,
            mut transforms,
            mut sprite_render,
            mut marked,
        ): Self::SystemData,
    ) {
        let mut marker_transform = None;
        for (e, transform, _, _) in (&*entities, &transforms, &selecteds, !&marked).join() {
            marker_transform = Some((e, transform.clone()));
        }
        // Mark selected entities
        if let Some((e, t)) = marker_transform {
            info!("Found selected element!");
            let marker_entity = entities
                .build_entity()
                .with(
                    selected_sprite_render.sprite_render.clone(),
                    &mut sprite_render,
                )
                .with(t.clone(), &mut transforms)
                .build();

            marked
                .insert(e, MarkedAsSelected::new(&marker_entity))
                .unwrap();
        }

        // Remove marker entity for unselected entities
        let mut marker_to_be_removed = None;
        for (e, _, mark) in (&*entities, !&selecteds, &marked).join() {
            let marker_entity = entities.entity(mark.index);
            marker_to_be_removed = Some(e);
            if entities.is_alive(marker_entity) {
                info!("Removing selection marker entity!");
                entities
                    .delete(marker_entity)
                    .expect("Unable to delete marker entity");
            }
        }
        if let Some(e) = marker_to_be_removed {
            info!("Removing a selection marker component!");
            marked.remove(e);
        };
    }
}

struct MouseSystem;

impl<'s> System<'s> for MouseSystem {
    type SystemData = (
        Entities<'s>,
        Read<'s, InputHandler<StringBindings>>,
        WriteStorage<'s, Transform>,
        WriteStorage<'s, UiTransform>,
        WriteStorage<'s, Selected>,
        WriteStorage<'s, SomeObject>,
        ReadExpect<'s, ScreenDimensions>,
        WriteStorage<'s, MarkedAsSelected>,
    );

    fn run(
        &mut self,
        (
            entities,
            input,
            mut transforms,
            mut ui_transforms,
            selected,
            mut some_objects,
            screen_dimension,
            marked_as_selected,
        ): Self::SystemData,
    ) {
        // Compute point where selected object ist ordered to
        for (transform, _, mut some_object) in (&transforms, &selected, &mut some_objects).join() {
            if let Some(pressed) = input.action_is_down("move") {
                if pressed {
                    let hidpi_factor = screen_dimension.hidpi_factor() as f32;
                    let (screen_size_x, screen_size_y) = (
                        screen_dimension.width() / hidpi_factor,
                        screen_dimension.height() / hidpi_factor,
                    );
                    some_object.ordered_to = match input.mouse_position() {
                        Some((x, y)) => Some((
                            (x / hidpi_factor) - (screen_size_x / 2.),
                            -(y / hidpi_factor) + (screen_size_y / 2.),
                        )),
                        None => None,
                    };
                    info!(
                        "Ordered object to move to position {:?} current transform position {:?}",
                        some_object.ordered_to,
                        transform.translation()
                    );
                }
            }
        }

        // Move transform and UiTransform if object is ordered to move
        for (transform, mut ui_transform, some_object) in
            (&mut transforms, &mut ui_transforms, &some_objects).join()
            {
                if let Some((target_pos_x, target_pos_y)) = some_object.ordered_to {
                    let movement_vec = (
                        target_pos_x - transform.translation().x,
                        target_pos_y - transform.translation().y,
                    );
                    let movement_length = 5. * (movement_vec.0.powi(2) + movement_vec.1.powi(2)).sqrt();
                    transform.append_translation_xyz(
                        movement_vec.0 / movement_length,
                        movement_vec.1 / movement_length,
                        0.,
                    );

                    ui_transform.local_x += (movement_vec.0 / movement_length) as f32;
                    ui_transform.local_y += (movement_vec.1 / movement_length) as f32;
                }
            }

        // Move marker for selected entities transform
        for (some_object, marked) in (&some_objects, &marked_as_selected).join() {
            if let Some((target_pos_x, target_pos_y)) = some_object.ordered_to {
                let marker_entity = entities.entity(marked.index);
                if entities.is_alive(marker_entity) {
                    let marker_transform = transforms
                        .entry(marker_entity)
                        .expect("No transform found for marker entity")
                        .or_insert(Transform::default());

                    let movement_vec = (
                        target_pos_x - marker_transform.translation().x,
                        target_pos_y - marker_transform.translation().y,
                    );

                    let movement_length =
                        5. * (movement_vec.0.powi(2) + movement_vec.1.powi(2)).sqrt();
                    marker_transform.append_translation_xyz(
                        movement_vec.0 / movement_length,
                        movement_vec.1 / movement_length,
                        0.,
                    );
                }
            }
        }
    }
}

fn load_sprite_sheet(world: &mut World) -> Handle<SpriteSheet> {
    let texture_handle = {
        let loader = world.read_resource::<Loader>();
        let texture_storage = world.read_resource::<AssetStorage<Texture>>();
        loader.load(
            "texture/spritesheet.png",
            ImageFormat::default(),
            (),
            &texture_storage,
        )
    };

    let loader = world.read_resource::<Loader>();
    let sprite_sheet_store = world.read_resource::<AssetStorage<SpriteSheet>>();
    loader.load(
        "texture/spritesheet.ron",
        SpriteSheetFormat(texture_handle),
        (),
        &sprite_sheet_store,
    )
}

struct Example;

impl SimpleState for Example {
    fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
        let StateData { world, .. } = data;
        let (width, height) = {
            let dim = world.read_resource::<ScreenDimensions>();
            (dim.width(), dim.height())
        };

        let mut camera_transform = Transform::default();
        camera_transform.set_translation_z(1.0);

        world
            .create_entity()
            .with(camera_transform)
            .with(Camera::standard_2d(width, height))
            .build();

        let sprite_sheet_handle = load_sprite_sheet(world);

        // Initialize left object
        let mut left_transform = Transform::default();
        left_transform.set_translation_xyz(0.0, 0.0, 0.0);

        let sprite_render = SpriteRender {
            sprite_sheet: sprite_sheet_handle.clone(),
            sprite_number: 0,
        };

        let ui_transform = UiTransform::new(
            "test".to_string(),
            Anchor::Middle,
            Anchor::Middle,
            0.,
            0.,
            0.,
            64.,
            64.,
        );

        world
            .create_entity()
            .with(sprite_render.clone())
            .with(left_transform)
            .with(ui_transform)
            .with(Selectable::<()>::new(0))
            .with(Interactable)
            .with(SomeObject::new())
            .build();

        // Initialize right object
        let mut right_transform = Transform::default();
        right_transform.set_translation_xyz(100.0, 100.0, 0.0);

        let right_sprite_render = SpriteRender {
            sprite_sheet: sprite_sheet_handle.clone(),
            sprite_number: 1,
        };

        let right_ui_transform = UiTransform::new(
            "test2".to_string(),
            Anchor::Middle,
            Anchor::Middle,
            100.,
            100.,
            0.,
            64.,
            64.,
        );

        world
            .create_entity()
            .with(right_sprite_render.clone())
            .with(right_transform)
            .with(right_ui_transform)
            .with(Selectable::<()>::new(0))
            .with(Interactable)
            .with(SomeObject::new())
            .build();

        // Initialize selected frame as resource
        world.add_resource(SelectedSpriteRender {
            sprite_render: SpriteRender {
                sprite_sheet: sprite_sheet_handle.clone(),
                sprite_number: 2,
            },
        });
    }

    fn update(&mut self, _data: &mut StateData<'_, GameData<'_, '_>>) -> SimpleTrans {
        Trans::None
    }
}

fn main() -> amethyst::Result<()> {
    amethyst::start_logger(Default::default());

    let app_root = application_root_dir()?;

    let display_config_path = app_root.join("src/resources/display_config.ron");

    let bindings_config_path = app_root.join("src/resources/bindings_config.ron");

    let resources = app_root.join("src/assets/");
    let game_data = GameDataBuilder::default()
        //.with_bundle(WindowBundle::from_config_path(display_config_path))?
        .with_bundle(
            InputBundle::<StringBindings>::new().with_bindings_from_file(bindings_config_path)?,
        )?
        .with_bundle(TransformBundle::new())?
        .with_bundle(UiBundle::<StringBindings>::new())?
        .with(MouseSystem, "mouse_system", &["input_system"])
        .with(MarkSelectedSystem, "mark_selected_system", &[])
        .with_bundle(
            RenderingBundle::<DefaultBackend>::new()
                // The RenderToWindow plugin provides all the scaffolding for opening a window and
                // drawing on it
                .with_plugin(
                    RenderToWindow::from_config_path(display_config_path)
                        .with_clear([0.34, 0.36, 0.52, 1.0]),
                )
                .with_plugin(RenderFlat2D::default())
                .with_plugin(RenderUi::default()),
        )?;
    let mut game = Application::new(resources, Example, game_data)?;
    game.run();
    Ok(())
}
