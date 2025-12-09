//! Handles a `world.fuzzy_resource_name_to_name` request coming from a client.
use bevy::{
    prelude::*,
    remote::{BrpError, BrpResult, RemoteMethodSystemId, RemoteMethods},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    brp::fuzzy_component_name_to_name::no_fuzzy_match_brp_error,
    component_inspection::ComponentMetadataMap, fuzzy_name_mapping::fuzzy_resource_name_to_id,
};

pub const METHOD: &str = "world.fuzzy_resource_name_to_name";

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Params {
    pub fuzzy_name: String,
    pub metadata_map: Option<ComponentMetadataMap>,
}

pub fn process_remote_request(In(params): In<Option<Value>>, world: &World) -> BrpResult {
    let Params {
        fuzzy_name,
        metadata_map,
    } = super::parse_some(params)?;
    match fuzzy_resource_name_to_id(world, &fuzzy_name) {
        Some(component_id) => {
            let metadata_map = metadata_map.unwrap_or(ComponentMetadataMap::generate(world));
            let component_metadata = metadata_map.get(&component_id);
            let Some(component_metadata) = component_metadata else {
                let index = component_id.index();
                return Err(BrpError::component_error(format!(
                    "Could not find metadata for component `{index}`"
                )));
            };
            Ok(serde_json::to_value(component_metadata.name.to_string())
                .map_err(BrpError::internal)?)
        }
        None => Err(no_fuzzy_match_brp_error(&fuzzy_name)),
    }
}
