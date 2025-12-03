//! Demonstrates how to inspect an out-of-process Bevy app
//! by sending BRP requests to it.
//!
//! Run this example with the `remote` feature enabled:
//! ```bash
//! cargo run --example brp_client --features="remote"
//! ```

use bevy::prelude::*;
use bevy::remote::BrpRequest;
use bevy::remote::builtin_methods::{BRP_QUERY_METHOD, BrpQuery, BrpQueryFilter, BrpQueryParams};
use bevy::remote::http::{DEFAULT_ADDR, DEFAULT_PORT};
use feathers_inspector::brp_methods::{self, BrpWorldInspectParams};
use feathers_inspector::component_inspection::{ComponentDetailLevel, ComponentInspectionSettings};
use feathers_inspector::entity_inspection::EntityInspectionSettings;

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
        .add_systems(Update, (inspect_all_entities_when_space_pressed,))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let instructions = "\
This is your client process, that connects to the Bevy app via BRP.
You can use the keyboard buttons to send BRP requests.
Output will be shown in the console.

Press `Space` to inspect all entities"
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
        for entity in entities {
            let inspection = helper::inspect_entity_cached(entity, &component_metadata, &brp_url.0);
            info!("{inspection}");
        }
    }
}

// Since BRP request and response handling are quite verbose,
// we define a helper module to contain the complexity.
mod helper {

    use feathers_inspector::{
        brp_methods::BrpWorldInspectCachedParams, component_inspection::ComponentMetadataMap,
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

    #[allow(dead_code)]
    pub fn inspect_entity(entity: Entity, url: &str) -> String {
        let brp_request = BrpRequest {
            jsonrpc: String::from("2.0"),
            method: brp_methods::BRP_WORLD_INSPECT_METHOD.to_string(),
            id: None,
            params: Some(
                serde_json::to_value(BrpWorldInspectParams {
                    entity,
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
}
