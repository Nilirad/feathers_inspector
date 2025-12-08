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
use feathers_inspector::component_inspection::{ComponentDetailLevel, ComponentInspectionSettings};
use feathers_inspector::entity_inspection::{
    EntityInspectionSettings, MultipleEntityInspectionSettings,
};
use feathers_inspector::resource_inspection::ResourceInspectionSettings;
use feathers_inspector::summary::SummarySettings;

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
                summarize_when_s_pressed,
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
Press 'M' to inspect the Sprite component type metadata
Press 'S' to obtain summary statistics"
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
        let query_params = BrpQueryParams {
            data: BrpQuery::default(),
            filter: BrpQueryFilter::default(),
            strict: false,
        };
        let entities = helper::query(query_params, &brp_url.0);
        let component_metadata = helper::generate_component_metadata_map(&brp_url.0);
        let settings = MultipleEntityInspectionSettings {
            entity_settings: EntityInspectionSettings {
                include_components: false,
                ..default()
            },
            ..default()
        };
        let inspections = inspect_multiple(entities, settings, component_metadata, &brp_url.0);
        for result in inspections {
            if let Ok(inspection) = result {
                info!("{inspection}");
            } else {
                warn!("Could not inspect an entity")
            }
        }
    }
}

fn inspect_specific_component_when_c_pressed(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    brp_url: Res<BrpUrl>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyC) {
        let query_params = BrpQueryParams {
            data: BrpQuery::default(),
            filter: BrpQueryFilter {
                with: vec![SPRITE_COMPONENT_NAME.to_string()],
                ..default()
            },
            strict: false,
        };
        let entities = helper::query(query_params, &brp_url.0);
        let settings = ComponentInspectionSettings {
            detail_level: ComponentDetailLevel::Values,
            full_type_names: true,
        };
        for entity in entities {
            let inspection = inspect_component(
                SPRITE_COMPONENT_NAME.to_string(),
                entity,
                settings,
                &brp_url.0,
            );
            info!("{inspection}");
        }
    }
}

fn inspect_resource_when_r_pressed(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    brp_url: Res<BrpUrl>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyR) {
        let settings = ResourceInspectionSettings {
            full_type_names: true,
        };
        let inspection = helper::inspect_resource(
            AMBIENT_LIGHT_COMPONENT_NAME.to_string(),
            settings,
            &brp_url.0,
        );
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
        for inspection in inspections {
            info!("{inspection}");
        }
    }
}

fn inspect_sprite_component_type_when_m_pressed(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    brp_url: Res<BrpUrl>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyM) {
        let inspection =
            helper::inspect_component_type(SPRITE_COMPONENT_NAME.to_string(), &brp_url.0);
        info!("{inspection}");
    }
}

fn summarize_when_s_pressed(keyboard_input: Res<ButtonInput<KeyCode>>, brp_url: Res<BrpUrl>) {
    if keyboard_input.just_pressed(KeyCode::KeyS) {
        let settings = SummarySettings::default();
        let summary = helper::summarize(settings, &brp_url.0);
        info!("{summary}");
    }
}

// Since BRP request and response handling are quite verbose,
// we define a helper module to contain the complexity.
// TODO: Helpers should return concrete types instead of a JSON string,
//       just like `generate_component_metadata` does.
mod helper {
    use feathers_inspector::{
        brp,
        component_inspection::{
            ComponentInspection, ComponentMetadataMap, ComponentTypeInspection,
        },
        entity_inspection::{
            EntityInspection, EntityInspectionError, MultipleEntityInspectionSettings,
        },
        resource_inspection::{ResourceInspection, ResourceInspectionSettings},
        summary::{SummarySettings, WorldSummary},
    };

    use super::*;

    pub fn query(params: BrpQueryParams, url: &str) -> Vec<Entity> {
        let query_entities_request = BrpRequest {
            jsonrpc: String::from("2.0"),
            method: String::from(BRP_QUERY_METHOD),
            id: None,
            params: Some(
                serde_json::to_value(params)
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
    pub fn inspect_entity(
        entity: Entity,
        settings: EntityInspectionSettings,
        url: &str,
    ) -> EntityInspection {
        let brp_request = BrpRequest {
            jsonrpc: String::from("2.0"),
            method: brp::inspect::METHOD.to_string(),
            id: None,
            params: Some(
                serde_json::to_value(brp::inspect::Params { entity, settings })
                    .expect("Unable to convert query parameters to a valid JSON value"),
            ),
        };
        let response = ureq::post(url)
            .send_json(brp_request)
            .expect("Failed to send JSON to server")
            .body_mut()
            .read_json::<serde_json::Value>()
            .expect("Failed to read JSON response");
        let result = response
            .get("result")
            .expect("Missing `result` field in JSON-RPC response");
        serde_json::from_value::<EntityInspection>(result.clone())
            .expect("Failed to deserialize `EntityInspection`")
    }

    #[allow(dead_code)]
    pub fn inspect_entity_cached(
        entity: Entity,
        metadata_map: &ComponentMetadataMap,
        settings: EntityInspectionSettings,
        url: &str,
    ) -> EntityInspection {
        let brp_request = BrpRequest {
            jsonrpc: String::from("2.0"),
            method: brp::inspect_cached::METHOD.to_string(),
            id: None,
            params: Some(
                serde_json::to_value(brp::inspect_cached::Params {
                    entity,
                    settings,
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
        let result = response
            .get("result")
            .expect("Missing `result` field in JSON-RPC response");
        serde_json::from_value::<EntityInspection>(result.clone())
            .expect("Failed to deserialize `EntityInspection`")
    }

    pub fn inspect_multiple(
        entities: impl IntoIterator<Item = Entity>,
        settings: MultipleEntityInspectionSettings,
        metadata_map: ComponentMetadataMap,
        url: &str,
    ) -> Vec<Result<EntityInspection, EntityInspectionError>> {
        let brp_request = BrpRequest {
            jsonrpc: String::from("2.0"),
            method: brp::inspect_multiple::METHOD.to_string(),
            id: None,
            params: Some(
                serde_json::to_value(brp::inspect_multiple::Params {
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
        let result = response
            .get("result")
            .expect("Missing `result` field in JSON-RPC response");
        serde_json::from_value::<Vec<Result<EntityInspection, EntityInspectionError>>>(
            result.clone(),
        )
        .expect("Failed to deserialize `Vec<Result<EntityInspection, EntityInspectionError>>`")
    }

    pub fn generate_component_metadata_map(url: &str) -> ComponentMetadataMap {
        let request = BrpRequest {
            jsonrpc: String::from("2.0"),
            method: brp::component_metadata_map_generate::METHOD.to_string(),
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
            .expect("Missing `result` field in JSON-RPC response");
        serde_json::from_value::<ComponentMetadataMap>(result.clone())
            .expect("Failed to deserialize `ComponentMetadataMap`")
    }

    pub fn inspect_component(
        component_type: String,
        entity: Entity,
        settings: ComponentInspectionSettings,
        url: &str,
    ) -> ComponentInspection {
        let request = BrpRequest {
            jsonrpc: String::from("2.0"),
            method: brp::inspect_component::METHOD.to_string(),
            id: None,
            params: Some(
                serde_json::to_value(brp::inspect_component::Params {
                    component_type,
                    entity,
                    settings,
                    metadata_map: None,
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
        let result = response
            .get("result")
            .expect("Missing `result` field in JSON-RPC response");
        serde_json::from_value::<ComponentInspection>(result.clone())
            .expect("Failed to deserialize `ComponentInspection`")
    }

    pub fn inspect_resource(
        component_type: String,
        settings: ResourceInspectionSettings,
        url: &str,
    ) -> ResourceInspection {
        let request = BrpRequest {
            jsonrpc: String::from("2.0"),
            method: brp::inspect_resource::METHOD.to_string(),
            id: None,
            params: Some(
                serde_json::to_value(brp::inspect_resource::Params {
                    component_type,
                    settings,
                    metadata_map: None,
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
        let result = response
            .get("result")
            .expect("Missing `result` field in JSON-RPC response");
        serde_json::from_value::<ResourceInspection>(result.clone())
            .expect("Failed to deserialize `ResourceInspection`")
    }

    pub fn inspect_all_resources(
        settings: ResourceInspectionSettings,
        url: &str,
    ) -> Vec<ResourceInspection> {
        let request = BrpRequest {
            jsonrpc: String::from("2.0"),
            method: brp::inspect_all_resources::METHOD.to_string(),
            id: None,
            params: Some(
                serde_json::to_value(brp::inspect_all_resources::Params { settings })
                    .expect("Unable to convert query parameters to a valid JSON value"),
            ),
        };
        let response = ureq::post(url)
            .send_json(request)
            .expect("Failed to send JSON to server")
            .body_mut()
            .read_json::<serde_json::Value>()
            .expect("Failed to read JSON response");
        let result = response
            .get("result")
            .expect("Missing `result` field in JSON-RPC response");
        serde_json::from_value::<Vec<ResourceInspection>>(result.clone())
            .expect("Failed to deserialize `Vec<ResourceInspection>`")
    }

    pub fn inspect_component_type(component_type: String, url: &str) -> ComponentTypeInspection {
        let request = BrpRequest {
            jsonrpc: String::from("2.0"),
            method: brp::inspect_component_type::METHOD.to_string(),
            id: None,
            params: Some(
                serde_json::to_value(brp::inspect_component_type::Params {
                    component_type,
                    metadata_map: None,
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
        let result = response
            .get("result")
            .expect("Missing `result` field in JSON-RPC response");
        serde_json::from_value::<ComponentTypeInspection>(result.clone())
            .expect("Failed to deserialize `ComponentTypeInspection`")
    }

    pub fn summarize(settings: SummarySettings, url: &str) -> WorldSummary {
        let request = BrpRequest {
            jsonrpc: String::from("2.0"),
            method: brp::summarize::METHOD.to_string(),
            id: None,
            params: Some(
                serde_json::to_value(brp::summarize::Params { settings })
                    .expect("Unable to convert query parameters to a valid JSON value"),
            ),
        };
        let response = ureq::post(url)
            .send_json(request)
            .expect("Failed to send JSON to server")
            .body_mut()
            .read_json::<serde_json::Value>()
            .expect("Failed to read JSON response");
        let result = response
            .get("result")
            .expect("Missing `result` field in JSON-RPC response");
        serde_json::from_value::<WorldSummary>(result.clone())
            .expect("Failed to deserialize `WorldSummary`")
    }
}
