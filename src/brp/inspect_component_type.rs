use bevy::{
    prelude::*,
    remote::{BrpError, BrpResult, RemoteMethodSystemId, RemoteMethods},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    component_inspection::{ComponentInspectionError, ComponentMetadataMap},
    extension_methods::WorldInspectionExtensionTrait,
};

pub const METHOD: &str = "world.inspect_component_type";

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
    pub component_type: String,
    pub metadata_map: Option<ComponentMetadataMap>,
}

/// Handles a `world.inspect_component_type` request coming from a client.
pub fn process_remote_request(In(params): In<Option<Value>>, world: &World) -> BrpResult {
    let Params {
        component_type,
        metadata_map,
    } = super::parse_some(params)?;
    let metadata_map = metadata_map.unwrap_or(ComponentMetadataMap::generate(world));
    let Some((component_id, _)) = super::component_type_to_metadata(&component_type, &metadata_map)
    else {
        return Err(BrpError::component_error(
            "Component not found in metadata: `{component_type}`",
        ));
    };
    match world.inspect_component_type_by_id(component_id) {
        Ok(inspection) => Ok(serde_json::to_value(inspection).map_err(BrpError::internal)?),
        Err(error) => match error {
            ComponentInspectionError::ComponentNotFound(component_id) => {
                let component_index = component_id.index().to_string();
                Err(BrpError::component_error(format!(
                    "Component not found: {component_index}"
                )))
            }
            ComponentInspectionError::ComponentNotRegistered(component_type_name) => {
                Err(BrpError::component_error(format!(
                    "Component not registered: {component_type_name}"
                )))
            }
            ComponentInspectionError::ComponentIdNotRegistered(component_id) => {
                let component_index = component_id.index().to_string();
                Err(BrpError::component_error(format!(
                    "Component not registered: {component_index}"
                )))
            }
        },
    }
}
