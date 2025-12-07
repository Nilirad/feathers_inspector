use bevy::{
    ecs::query::QueryEntityError,
    prelude::*,
    remote::{BrpError, BrpResult, RemoteMethodSystemId, RemoteMethods},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    entity_inspection::{EntityInspectionError, EntityInspectionSettings},
    extension_methods::WorldInspectionExtensionTrait,
};

pub const METHOD: &str = "world.inspect";

pub(crate) struct VerbPlugin;

impl Plugin for VerbPlugin {
    fn build(&self, app: &mut App) {
        let world = app.world_mut();
        let system_id = world.register_system(process_remote_request);
        let mut remote_methods = world
            .get_resource_mut::<RemoteMethods>()
            .expect("`RemotePlugin` must be present");
        remote_methods.insert(METHOD, RemoteMethodSystemId::Instant(system_id));
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Params {
    pub entity: Entity,
    pub settings: EntityInspectionSettings,
}

/// Handles a `world.inspect` request coming from a client.
pub fn process_remote_request(In(params): In<Option<Value>>, world: &World) -> BrpResult {
    let Params { entity, settings } = super::parse_some(params)?;
    let entity_inspection = world.inspect(entity, settings);
    match entity_inspection {
        Ok(entity_inspection) => {
            serde_json::to_value(entity_inspection).map_err(BrpError::internal)
        }
        Err(inspection_error) => match inspection_error {
            EntityInspectionError::EntityNotFound(_) => Err(BrpError::entity_not_found(entity)),
            EntityInspectionError::UnexpectedQueryError(query_entity_error) => {
                match query_entity_error {
                    QueryEntityError::QueryDoesNotMatch(_, _) => Err(BrpError::internal(
                        "Reached invalid state: `QueryDoesNotMatch` on `SpawnDetails`",
                    )),
                    QueryEntityError::EntityDoesNotExist(_) => {
                        Err(BrpError::entity_not_found(entity))
                    }
                    QueryEntityError::AliasedMutability(_) => Err(BrpError::internal(
                        "Reached invalid state: `AliasedMutability`",
                    )),
                }
            }
        },
    }
}
