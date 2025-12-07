//! Demonstrates how to inspect an out-of-process Bevy app
//! by sending BRP requests to it.
//!
//! Run this example with the `remote` feature enabled:
//! ```bash
//! cargo run --example brp_client --features="remote"
//! ```
// TODO: Generate component metadata map once and store it in a resource.

use bevy::prelude::*;
use bevy::remote::BrpRequest;
use bevy::remote::builtin_methods::{BRP_QUERY_METHOD, BrpQuery, BrpQueryFilter, BrpQueryParams};
use bevy::remote::http::{DEFAULT_ADDR, DEFAULT_PORT};
use feathers_inspector::brp_methods::{self, BrpWorldInspectParams};
use feathers_inspector::component_inspection::{ComponentDetailLevel, ComponentInspectionSettings};
use feathers_inspector::entity_inspection::{
    EntityInspectionSettings, MultipleEntityInspectionSettings,
};
use feathers_inspector::resource_inspection::ResourceInspectionSettings;

use crate::helper::{inspect_component, inspect_multiple};

const SPRITE_COMPONENT_NAME: &str = "bevy_sprite::sprite::Sprite";
const AMBIENT_LIGHT_COMPONENT_NAME: &str = "bevy_light::ambient_light::AmbientLight";

#[derive(Resource, Debug)]
struct BrpUrl(String);

impl Default for BrpUrl {
    fn default() -> Self {
        let host_part = format!("{DEFAULT_ADDR}:{DEFAULT_PORT}");
        Self(format!("http://{host_part}/"))
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<BrpUrl>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                inspect_all_entities_when_space_pressed,
                inspect_specific_component_when_c_pressed,
                inspect_resource_when_r_pressed,
                inspect_all_resources_when_a_pressed,
                inspect_sprite_component_type_when_m_pressed,
            ),
        )
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let instructions = "\
This is your client process, that connects to the Bevy app via BRP.
You can use the keyboard buttons to send BRP requests.
Output will be shown in the console.

Press `Space` to inspect all entities
Press 'C' to inspect the Sprite component on all Sprite entities
Press 'R' to inspect the AmbientLight resource
Press 'A' to inspect all resources
Press 'M' to inspect the Sprite component type metadata"
        .to_string();

    commands.spawn((
        Text::new(instructions),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

fn inspect_all_entities_when_space_pressed(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    brp_url: Res<BrpUrl>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        let entities = helper::query_all_entities(&brp_url.0);
        let component_metadata = helper::generate_component_metadata(&brp_url.0);
        let settings = MultipleEntityInspectionSettings {
            entity_settings: EntityInspectionSettings {
                include_components: false,
                ..default()
            },
            ..default()
        };
        let inspection = inspect_multiple(entities, settings, component_metadata, &brp_url.0);
        info!("{inspection}");
    }
}

fn inspect_specific_component_when_c_pressed(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    brp_url: Res<BrpUrl>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyC) {
        let entities = helper::query_sprite_entities(&brp_url.0);
        let settings = ComponentInspectionSettings {
            detail_level: ComponentDetailLevel::Values,
            full_type_names: true,
        };
        let component_metadata = helper::generate_component_metadata(&brp_url.0);
        let (component_id, sprite_metadata) = component_metadata
            .map
            .iter()
            .find_map(|(id, meta)| {
                let full = meta.name.to_string();
                (full == SPRITE_COMPONENT_NAME).then_some((*id, meta))
            })
            .expect("Sprite metadata not found in remote world");
        for entity in entities {
            let inspection =
                inspect_component(component_id, entity, sprite_metadata, settings, &brp_url.0);
            info!("{inspection}");
        }
    }
}

fn inspect_resource_when_r_pressed(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    brp_url: Res<BrpUrl>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyR) {
        let component_metadata = helper::generate_component_metadata(&brp_url.0);
        let settings = ResourceInspectionSettings {
            full_type_names: true,
        };
        let component_id = component_metadata
            .map
            .iter()
            .find_map(|(id, meta)| {
                let full = meta.name.to_string();
                (full == AMBIENT_LIGHT_COMPONENT_NAME).then_some(*id)
            })
            .expect("`AmbientLight` metadata not found in remote world");
        let inspection = helper::inspect_resource(component_id, settings, &brp_url.0);
        info!("{inspection}");
    }
}

fn inspect_all_resources_when_a_pressed(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    brp_url: Res<BrpUrl>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyA) {
        let settings = ResourceInspectionSettings {
            full_type_names: false,
        };
        let inspections = helper::inspect_all_resources(settings, &brp_url.0);
        info!("{inspections}");
    }
}

fn inspect_sprite_component_type_when_m_pressed(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    brp_url: Res<BrpUrl>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyM) {
        let component_metadata = helper::generate_component_metadata(&brp_url.0);
        let component_id = component_metadata
            .map
            .iter()
            .find_map(|(id, meta)| {
                let full = meta.name.to_string();
                (full == SPRITE_COMPONENT_NAME).then_some(*id)
            })
            .expect("Sprite metadata not found in remote world");
        let inspection = helper::inspect_component_type(component_id, &brp_url.0);
        info!("{inspection}");
    }
}

// Since BRP request and response handling are quite verbose,
// we define a helper module to contain the complexity.
// TODO: Helpers should return concrete types instead of a JSON string,
//       just like `generate_component_metadata` does.
mod helper {
    use bevy::ecs::component::ComponentId;
    use feathers_inspector::{
        brp_methods::{
            BrpWorldInspectAllResourcesParams, BrpWorldInspectCachedParams,
            BrpWorldInspectComponentByIdParams, BrpWorldInspectComponentTypeByIdParams,
            BrpWorldInspectMultipleParams, BrpWorldInspectResourceByIdParams,
        },
        component_inspection::{ComponentMetadataMap, ComponentTypeMetadata},
        entity_inspection::MultipleEntityInspectionSettings,
        resource_inspection::ResourceInspectionSettings,
    };

    use super::*;

    pub fn query_all_entities(url: &str) -> Vec<Entity> {
        let query_entities_request = BrpRequest {
            jsonrpc: String::from("2.0"),
            method: String::from(BRP_QUERY_METHOD),
            id: None,
            params: Some(
                serde_json::to_value(BrpQueryParams {
                    data: BrpQuery::default(),
                    filter: BrpQueryFilter::default(),
                    strict: false,
                })
                .expect("Unable to convert query parameters to a valid JSON value"),
            ),
        };
        let response = ureq::post(url)
            .send_json(query_entities_request)
            .expect("Failed to send JSON to server")
            .body_mut()
            .read_json::<serde_json::Value>()
            .expect("Failed to read JSON response");
        response["result"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item["entity"].as_u64())
                    .map(Entity::from_bits)
                    .collect::<Vec<Entity>>()
            })
            .unwrap_or_default()
    }

    pub fn query_sprite_entities(url: &str) -> Vec<Entity> {
        let query_entities_request = BrpRequest {
            jsonrpc: String::from("2.0"),
            method: String::from(BRP_QUERY_METHOD),
            id: None,
            params: Some(
                serde_json::to_value(BrpQueryParams {
                    data: BrpQuery::default(),
                    filter: BrpQueryFilter {
                        with: vec![SPRITE_COMPONENT_NAME.to_string()],
                        ..default()
                    },
                    strict: false,
                })
                .expect("Unable to convert query parameters to a valid JSON value"),
            ),
        };
        let response = ureq::post(url)
            .send_json(query_entities_request)
            .expect("Failed to send JSON to server")
            .body_mut()
            .read_json::<serde_json::Value>()
            .expect("Failed to read JSON response");
        response["result"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item["entity"].as_u64())
                    .map(Entity::from_bits)
                    .collect::<Vec<Entity>>()
            })
            .unwrap_or_default()
    }

    #[allow(dead_code)]
    pub fn inspect_entity(entity: Entity, url: &str) -> String {
        let brp_request = BrpRequest {
            jsonrpc: String::from("2.0"),
            method: brp_methods::BRP_WORLD_INSPECT_METHOD.to_string(),
            id: None,
            params: Some(
                serde_json::to_value(BrpWorldInspectParams {
                    entity,
                    // TODO: Parametrize `EntityInspectionSettings`.
                    settings: EntityInspectionSettings {
                        include_components: false,
                        component_settings: ComponentInspectionSettings {
                            detail_level: ComponentDetailLevel::Values,
                            full_type_names: true,
                        },
                    },
                })
                .expect("Unable to convert query parameters to a valid JSON value"),
            ),
        };
        let response = ureq::post(url)
            .send_json(brp_request)
            .expect("Failed to send JSON to server")
            .body_mut()
            .read_json::<serde_json::Value>()
            .expect("Failed to read JSON response");
        response.to_string()
    }

    #[allow(dead_code)]
    pub fn inspect_entity_cached(
        entity: Entity,
        metadata_map: &ComponentMetadataMap,
        url: &str,
    ) -> String {
        let brp_request = BrpRequest {
            jsonrpc: String::from("2.0"),
            method: brp_methods::BRP_WORLD_INSPECT_CACHED_METHOD.to_string(),
            id: None,
            params: Some(
                serde_json::to_value(BrpWorldInspectCachedParams {
                    entity,
                    // TODO: Parametrize `EntityInspectionSettings`.
                    settings: EntityInspectionSettings {
                        include_components: false,
                        component_settings: ComponentInspectionSettings {
                            detail_level: ComponentDetailLevel::Values,
                            full_type_names: true,
                        },
                    },
                    metadata_map: metadata_map.clone(),
                })
                .expect("Unable to convert query parameters to a valid JSON value"),
            ),
        };
        let response = ureq::post(url)
            .send_json(brp_request)
            .expect("Failed to send JSON to server")
            .body_mut()
            .read_json::<serde_json::Value>()
            .expect("Failed to read JSON response");
        response.to_string()
    }

    pub fn inspect_multiple(
        entities: impl IntoIterator<Item = Entity>,
        settings: MultipleEntityInspectionSettings,
        metadata_map: ComponentMetadataMap,
        url: &str,
    ) -> String {
        let brp_request = BrpRequest {
            jsonrpc: String::from("2.0"),
            method: brp_methods::BRP_WORLD_INSPECT_MULTIPLE_METHOD.to_string(),
            id: None,
            params: Some(
                serde_json::to_value(BrpWorldInspectMultipleParams {
                    entities: entities.into_iter().collect::<Vec<Entity>>(),
                    settings,
                    metadata_map,
                })
                .expect("Unable to convert query parameters to a valid JSON value"),
            ),
        };
        let response = ureq::post(url)
            .send_json(brp_request)
            .expect("Failed to send JSON to server")
            .body_mut()
            .read_json::<serde_json::Value>()
            .expect("Failed to read JSON response");
        response.to_string()
    }

    pub fn generate_component_metadata(url: &str) -> ComponentMetadataMap {
        let request = BrpRequest {
            jsonrpc: String::from("2.0"),
            method: brp_methods::BRP_COMPONENT_METADATA_MAP_GENERATE_METHOD.to_string(),
            id: None,
            params: None,
        };
        let response = ureq::post(url)
            .send_json(request)
            .expect("Failed to send JSON to server")
            .body_mut()
            .read_json::<serde_json::Value>()
            .expect("Failed to read JSON response");
        let result = response
            .get("result")
            .expect("Missing 'result' field in JSON-RPC response");
        serde_json::from_value::<ComponentMetadataMap>(result.clone())
            .expect("Failed to deserialize `ComponentMetadataMap`")
    }

    pub fn inspect_component(
        component_id: ComponentId,
        entity: Entity,
        metadata: &ComponentTypeMetadata,
        settings: ComponentInspectionSettings,
        url: &str,
    ) -> String {
        let request = BrpRequest {
            jsonrpc: String::from("2.0"),
            method: brp_methods::BRP_WORLD_INSPECT_COMPONENT_BY_ID_METHOD.to_string(),
            id: None,
            params: Some(
                serde_json::to_value(BrpWorldInspectComponentByIdParams {
                    component_id,
                    entity,
                    metadata: metadata.clone(),
                    settings,
                })
                .expect("Unable to convert query parameters to a valid JSON value"),
            ),
        };
        let response = ureq::post(url)
            .send_json(request)
            .expect("Failed to send JSON to server")
            .body_mut()
            .read_json::<serde_json::Value>()
            .expect("Failed to read JSON response");
        response.to_string()
    }

    pub fn inspect_resource(
        component_id: ComponentId,
        settings: ResourceInspectionSettings,
        url: &str,
    ) -> String {
        let request = BrpRequest {
            jsonrpc: String::from("2.0"),
            method: brp_methods::BRP_WORLD_INSPECT_RESOURCE_BY_ID_METHOD.to_string(),
            id: None,
            params: Some(
                serde_json::to_value(BrpWorldInspectResourceByIdParams {
                    component_id,
                    settings,
                })
                .expect("Unable to convert query parameters to a valid JSON value"),
            ),
        };
        let response = ureq::post(url)
            .send_json(request)
            .expect("Failed to send JSON to server")
            .body_mut()
            .read_json::<serde_json::Value>()
            .expect("Failed to read JSON response");
        response.to_string()
    }

    pub fn inspect_all_resources(settings: ResourceInspectionSettings, url: &str) -> String {
        let request = BrpRequest {
            jsonrpc: String::from("2.0"),
            method: brp_methods::BRP_WORLD_INSPECT_ALL_RESOURCES_METHOD.to_string(),
            id: None,
            params: Some(
                serde_json::to_value(BrpWorldInspectAllResourcesParams { settings })
                    .expect("Unable to convert query parameters to a valid JSON value"),
            ),
        };
        let response = ureq::post(url)
            .send_json(request)
            .expect("Failed to send JSON to server")
            .body_mut()
            .read_json::<serde_json::Value>()
            .expect("Failed to read JSON response");
        response.to_string()
    }

    pub fn inspect_component_type(component_id: ComponentId, url: &str) -> String {
        let request = BrpRequest {
            jsonrpc: String::from("2.0"),
            method: brp_methods::BRP_WORLD_INSPECT_COMPONENT_TYPE_BY_ID_METHOD.to_string(),
            id: None,
            params: Some(
                serde_json::to_value(BrpWorldInspectComponentTypeByIdParams { component_id })
                    .expect("Unable to convert query parameters to a valid JSON value"),
            ),
        };
        let response = ureq::post(url)
            .send_json(request)
            .expect("Failed to send JSON to server")
            .body_mut()
            .read_json::<serde_json::Value>()
            .expect("Failed to read JSON response");
        response.to_string()
    }
}
