//! Provides a plugin that adds custom BRP methods for this library.
//!
//! To remotely use [`World`] and [`Commands`] methods defined in this crate,
//! set up the BRP server in your Bevy app
//! according to [`bevy::remote`]'s documentation.
//! Then, register the custom methods by adding the [`InspectorBrpPlugin`].
//! Now you can send inspector requests via BRP to your app and get a response.
//!
//! Refer to the constants defined in this module
//! to understand the names of the registered methods.

use bevy::{
    ecs::query::QueryEntityError,
    prelude::*,
    remote::{BrpError, BrpResult, RemoteMethodSystemId, RemoteMethods, error_codes},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    component_inspection::ComponentMetadataMap,
    entity_inspection::{
        EntityInspectionError, EntityInspectionSettings, MultipleEntityInspectionSettings,
    },
    extension_methods::WorldInspectionExtensionTrait,
};

pub const BRP_WORLD_INSPECT_METHOD: &str = "world.inspect";
pub const BRP_WORLD_INSPECT_CACHED_METHOD: &str = "world.inspect_cached";
pub const BRP_WORLD_INSPECT_MULTIPLE_METHOD: &str = "world.inspect_multiple";
pub const BRP_WORLD_INSPECT_COMPONENT_BY_ID_METHOD: &str = "world.inspect_component_by_id";
pub const BRP_WORLD_INSPECT_RESOURCE_BY_ID_METHOD: &str = "world.inspect_resource_by_id";
pub const BRP_WORLD_INSPECT_ALL_RESOURCES_METHOD: &str = "world.inspect_all_resources";
pub const BRP_WORLD_INSPECT_COMPONENT_TYPE_BY_ID_METHOD: &str =
    "world.inspect_component_type_by_id";
pub const BRP_COMPONENT_METADATA_MAP_GENERATE_METHOD: &str = "component_metadata_map.generate";

/// A helper function used to parse a `serde_json::Value`.
fn parse<T: for<'de> Deserialize<'de>>(value: Value) -> Result<T, BrpError> {
    serde_json::from_value(value).map_err(|err| BrpError {
        code: error_codes::INVALID_PARAMS,
        message: err.to_string(),
        data: None,
    })
}

/// A helper function used to parse a `serde_json::Value` wrapped in an `Option`.
fn parse_some<T: for<'de> Deserialize<'de>>(value: Option<Value>) -> Result<T, BrpError> {
    match value {
        Some(value) => parse(value),
        None => Err(BrpError {
            code: error_codes::INVALID_PARAMS,
            message: String::from("Params not provided"),
            data: None,
        }),
    }
}

/// Provides inspection methods defined in this crate
/// to be called via BRP.
///
/// ## Panics
///
/// This plugin assumes [`RemotePlugin`] is already added,
/// and will panic otherwise.
///
/// [`RemotePlugin`]: bevy::remote::RemotePlugin
pub struct InspectorBrpPlugin;

impl Plugin for InspectorBrpPlugin {
    fn build(&self, app: &mut App) {
        let world = app.world_mut();

        let world_inspect_id = world.register_system(process_remote_world_inspect_request);
        let world_inspect_cached_id =
            world.register_system(process_remote_world_inspect_cached_request);
        let world_inspect_multiple_id =
            world.register_system(process_remote_world_inspect_multiple_request);
        let world_inspect_component_by_id_id =
            world.register_system(process_remote_world_inspect_component_by_id_request);
        let world_inspect_resource_by_id_id =
            world.register_system(process_remote_world_inspect_resource_by_id_request);
        let world_inspect_all_resources_id =
            world.register_system(process_remote_world_inspect_all_resources_request);
        let world_inspect_component_type_by_id_id =
            world.register_system(process_remote_world_inspect_component_type_by_id_request);
        let component_metadata_map_generate_id =
            world.register_system(process_remote_component_metadata_map_generate_request);

        // Avoids adding `RemotePlugin` by design,
        // since users might also want to add it themselves for other purposes.
        let mut remote_methods = world
            .get_resource_mut::<RemoteMethods>()
            .expect("`RemotePlugin` must be present");

        remote_methods.insert(
            BRP_WORLD_INSPECT_METHOD,
            RemoteMethodSystemId::Instant(world_inspect_id),
        );
        remote_methods.insert(
            BRP_WORLD_INSPECT_CACHED_METHOD,
            RemoteMethodSystemId::Instant(world_inspect_cached_id),
        );
        remote_methods.insert(
            BRP_WORLD_INSPECT_MULTIPLE_METHOD,
            RemoteMethodSystemId::Instant(world_inspect_multiple_id),
        );
        remote_methods.insert(
            BRP_WORLD_INSPECT_COMPONENT_BY_ID_METHOD,
            RemoteMethodSystemId::Instant(world_inspect_component_by_id_id),
        );
        remote_methods.insert(
            BRP_WORLD_INSPECT_RESOURCE_BY_ID_METHOD,
            RemoteMethodSystemId::Instant(world_inspect_resource_by_id_id),
        );
        remote_methods.insert(
            BRP_WORLD_INSPECT_ALL_RESOURCES_METHOD,
            RemoteMethodSystemId::Instant(world_inspect_all_resources_id),
        );
        remote_methods.insert(
            BRP_WORLD_INSPECT_COMPONENT_TYPE_BY_ID_METHOD,
            RemoteMethodSystemId::Instant(world_inspect_component_type_by_id_id),
        );
        remote_methods.insert(
            BRP_COMPONENT_METADATA_MAP_GENERATE_METHOD,
            RemoteMethodSystemId::Instant(component_metadata_map_generate_id),
        );
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpWorldInspectParams {
    pub entity: Entity,
    pub settings: EntityInspectionSettings,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpWorldInspectResponse;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BrpWorldInspectCachedParams {
    pub entity: Entity,
    pub settings: EntityInspectionSettings,
    // PERF: Use reference instead, since struct is heavy.
    pub metadata_map: ComponentMetadataMap,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpWorldInspectCachedResponse;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BrpWorldInspectMultipleParams {
    pub entities: Vec<Entity>,
    pub settings: MultipleEntityInspectionSettings,
    // PERF: Use reference instead, since struct is heavy.
    pub metadata_map: ComponentMetadataMap,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpWorldInspectMultipleResponse;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpWorldInspectComponentByIdParams;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpWorldInspectComponentByIdResponse;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpWorldInspectResourceByIdParams;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpWorldInspectResourceByIdResponse;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpWorldInspectAllResourcesParams;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpWorldInspectAllResourcesResponse;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpWorldInspectComponentTypeByIdParams;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpWorldInspectComponentTypeByIdResponse;

fn inspect_cached_brp(
    world: &World,
    entity: Entity,
    settings: &EntityInspectionSettings,
    metadata_map: &ComponentMetadataMap,
) -> std::result::Result<Value, BrpError> {
    let entity_inspection = world.inspect_cached(entity, &settings, &metadata_map);
    match entity_inspection {
        Ok(entity_inspection) => {
            serde_json::to_value(entity_inspection).map_err(BrpError::internal)
        }
        Err(inspection_error) => match inspection_error {
            EntityInspectionError::EntityNotFound(_) => Err(BrpError::entity_not_found(entity)),
            EntityInspectionError::UnexpectedQueryError(query_entity_error) => {
                match query_entity_error {
                    QueryEntityError::QueryDoesNotMatch(_, _) => unreachable!(),
                    QueryEntityError::EntityDoesNotExist(_) => {
                        Err(BrpError::entity_not_found(entity))
                    }
                    QueryEntityError::AliasedMutability(_) => unreachable!(),
                }
            }
        },
    }
}

/// Handles a `world.inspect` request coming from a client.
pub fn process_remote_world_inspect_request(
    In(params): In<Option<Value>>,
    world: &World,
) -> BrpResult {
    let BrpWorldInspectParams { entity, settings } = parse_some(params)?;
    let metadata_map = ComponentMetadataMap::for_entity(world, entity);
    inspect_cached_brp(world, entity, &settings, &metadata_map)
}

/// Handles a `world.inspect_cached` request coming from a client.
pub fn process_remote_world_inspect_cached_request(
    In(params): In<Option<Value>>,
    world: &World,
) -> BrpResult {
    let BrpWorldInspectCachedParams {
        entity,
        settings,
        metadata_map,
    } = parse_some(params)?;
    inspect_cached_brp(world, entity, &settings, &metadata_map)
}

/// Handles a `world.inspect_multiple` request coming from a client.
pub fn process_remote_world_inspect_multiple_request(
    In(params): In<Option<Value>>,
    world: &World,
) -> BrpResult {
    let BrpWorldInspectMultipleParams {
        entities,
        settings,
        mut metadata_map,
    } = parse_some(params)?;
    let inspection = world.inspect_multiple(entities, settings, &mut metadata_map);
    serde_json::to_value(inspection).map_err(BrpError::internal)
}

/// Handles a `world.inspect_component_by_id` request coming from a client.
pub fn process_remote_world_inspect_component_by_id_request(
    In(_params): In<Option<Value>>,
    _world: &World,
) -> BrpResult {
    let response = "called `world.inspect_component_by_id` handler successfully.";
    serde_json::to_value(response).map_err(BrpError::internal)
}

/// Handles a `world.inspect_resource_by_id` request coming from a client.
pub fn process_remote_world_inspect_resource_by_id_request(
    In(_params): In<Option<Value>>,
    _world: &World,
) -> BrpResult {
    let response = "called `world.inspect_resource_by_id` handler successfully.";
    serde_json::to_value(response).map_err(BrpError::internal)
}

/// Handles a `world.inspect_all_resources` request coming from a client.
pub fn process_remote_world_inspect_all_resources_request(
    In(_params): In<Option<Value>>,
    _world: &World,
) -> BrpResult {
    let response = "called `world.inspect_all_resources` handler successfully.";
    serde_json::to_value(response).map_err(BrpError::internal)
}

/// Handles a `world.inspect_component_type_by_id` request coming from a client.
pub fn process_remote_world_inspect_component_type_by_id_request(
    In(_params): In<Option<Value>>,
    _world: &World,
) -> BrpResult {
    let response = "called `world.inspect_component_type_by_id` handler successfully.";
    serde_json::to_value(response).map_err(BrpError::internal)
}

/// Handles a `component_metadata_map.generate` request coming from a client.
pub fn process_remote_component_metadata_map_generate_request(
    In(_params): In<Option<Value>>,
    world: &World,
) -> BrpResult {
    let metadata_map = ComponentMetadataMap::generate(world);
    serde_json::to_value(metadata_map).map_err(BrpError::internal)
}
