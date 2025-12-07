use bevy::{
    ecs::component::ComponentId,
    prelude::*,
    remote::{BrpError, BrpResult, RemoteMethodSystemId, RemoteMethods},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    component_inspection::{
        ComponentInspectionError, ComponentInspectionSettings, ComponentTypeMetadata,
    },
    extension_methods::WorldInspectionExtensionTrait,
};

pub const METHOD: &str = "world.inspect_component_by_id";

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
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::serde_conversions::component_id")
    )]
    pub component_id: ComponentId,
    pub entity: Entity,
    pub metadata: ComponentTypeMetadata,
    pub settings: ComponentInspectionSettings,
}

/// Handles a `world.inspect_component_by_id` request coming from a client.
pub fn process_remote_request(In(params): In<Option<Value>>, world: &World) -> BrpResult {
    let Params {
        component_id,
        entity,
        metadata,
        settings,
    } = super::parse_some(params)?;
    match world.inspect_component_by_id(component_id, entity, &metadata, settings) {
        Ok(inspection) => Ok(serde_json::to_value(inspection).map_err(BrpError::internal)?),
        Err(error) => match error {
            // TODO: Use component name instead of index.
            ComponentInspectionError::ComponentNotFound(component_id) => Err(
                BrpError::component_not_present(&component_id.index().to_string(), entity),
            ),
            ComponentInspectionError::ComponentNotRegistered(component_name) => Err(
                BrpError::component_error(format!("Component not registered: {component_name}")),
            ),
            ComponentInspectionError::ComponentIdNotRegistered(component_id) => Err(
                BrpError::component_error(format!("Component id not registered: {component_id:?}")),
            ),
        },
    }
}
