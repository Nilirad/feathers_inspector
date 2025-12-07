use bevy::{
    ecs::component::ComponentId,
    prelude::*,
    remote::{BrpError, BrpResult, RemoteMethodSystemId, RemoteMethods},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    extension_methods::WorldInspectionExtensionTrait,
    resource_inspection::{ResourceInspectionError, ResourceInspectionSettings},
};

pub const METHOD: &str = "world.inspect_resource_by_id";

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
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::serde_conversions::component_id")
    )]
    pub component_id: ComponentId,
    pub settings: ResourceInspectionSettings,
}

/// Handles a `world.inspect_resource_by_id` request coming from a client.
pub fn process_remote_request(In(params): In<Option<Value>>, world: &World) -> BrpResult {
    let Params {
        component_id,
        settings,
    } = super::parse_some(params)?;
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
