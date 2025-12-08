use bevy::{
    prelude::*,
    remote::{BrpError, BrpResult, RemoteMethodSystemId, RemoteMethods},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    component_inspection::ComponentMetadataMap,
    extension_methods::WorldInspectionExtensionTrait,
    resource_inspection::{ResourceInspectionError, ResourceInspectionSettings},
};

pub const METHOD: &str = "world.inspect_resource";

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
    pub settings: ResourceInspectionSettings,
    pub metadata_map: Option<ComponentMetadataMap>,
}

/// Handles a `world.inspect_resource_by_id` request coming from a client.
pub fn process_remote_request(In(params): In<Option<Value>>, world: &World) -> BrpResult {
    let Params {
        component_type,
        settings,
        metadata_map,
    } = super::parse_some(params)?;
    let metadata_map = metadata_map.unwrap_or(ComponentMetadataMap::generate(world));
    let Some((component_id, _)) = super::component_type_to_metadata(&component_type, &metadata_map)
    else {
        return Err(BrpError::component_error(
            "Component not found in metadata: `{component_type}`",
        ));
    };
    match world.inspect_resource_by_id(component_id, settings) {
        Ok(inspection) => Ok(serde_json::to_value(inspection).map_err(BrpError::internal)?),
        Err(error) => match error {
            ResourceInspectionError::ResourceNotRegistered(type_name) => Err(
                BrpError::resource_error(format!("Resource not registered: {type_name}")),
            ),
            ResourceInspectionError::ResourceNotFound(component_id) => Err(
                BrpError::resource_not_present(&format!("Resource not found: {component_id:?}")),
            ),
        },
    }
}
