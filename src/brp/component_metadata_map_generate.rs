//! Handles a `component_metadata_map.generate` request coming from a client.
use bevy::{
    prelude::*,
    remote::{BrpError, BrpResult, RemoteMethodSystemId, RemoteMethods},
};
use serde_json::Value;

use crate::component_inspection::ComponentMetadataMap;

pub const METHOD: &str = "component_metadata_map.generate";

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

pub fn process_remote_request(In(_params): In<Option<Value>>, world: &World) -> BrpResult {
    let metadata_map = ComponentMetadataMap::generate(world);
    serde_json::to_value(metadata_map).map_err(BrpError::internal)
}
