//! Handles a `world.inspect_all_resources` request coming from a client.
use bevy::{
    prelude::*,
    remote::{BrpError, BrpResult, RemoteMethodSystemId, RemoteMethods},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    extension_methods::WorldInspectionExtensionTrait,
    resource_inspection::ResourceInspectionSettings,
};

pub const METHOD: &str = "world.inspect_all_resources";

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
    pub settings: ResourceInspectionSettings,
}

pub fn process_remote_request(In(params): In<Option<Value>>, world: &World) -> BrpResult {
    let Params { settings } = super::parse_some(params)?;
    let inspection = world.inspect_all_resources(settings);
    serde_json::to_value(inspection).map_err(BrpError::internal)
}
