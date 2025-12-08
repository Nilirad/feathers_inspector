//! Provides a plugin that adds custom BRP verbs
//! for methods defined in this library.
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
    ecs::component::ComponentId,
    prelude::*,
    remote::{BrpError, error_codes},
};
use serde::Deserialize;
use serde_json::Value;

use crate::component_inspection::{ComponentMetadataMap, ComponentTypeMetadata};

pub mod component_metadata_map_generate;
pub mod inspect;
pub mod inspect_all_resources;
pub mod inspect_cached;
pub mod inspect_component;
pub mod inspect_component_type;
pub mod inspect_multiple;
pub mod inspect_resource;
pub mod summarize;

/// Provides BRP verbs for calling functions and methods defined in this crate.
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
        app.add_plugins((
            component_metadata_map_generate::VerbPlugin,
            inspect::VerbPlugin,
            inspect_all_resources::VerbPlugin,
            inspect_cached::VerbPlugin,
            inspect_component::VerbPlugin,
            inspect_component_type::VerbPlugin,
            inspect_multiple::VerbPlugin,
            inspect_resource::VerbPlugin,
            summarize::VerbPlugin,
        ));
    }
}

/// A helper function used to parse a `serde_json::Value`.
// NOTE: This function was copied from the homonymous function in `bevy_remote::builtin_methods`.
//       Remove once https://github.com/bevyengine/bevy/pull/22005 is merged and released.
fn parse<T: for<'de> Deserialize<'de>>(value: Value) -> Result<T, BrpError> {
    serde_json::from_value(value).map_err(|err| BrpError {
        code: error_codes::INVALID_PARAMS,
        message: err.to_string(),
        data: None,
    })
}

/// A helper function used to parse a `serde_json::Value` wrapped in an `Option`.
// NOTE: This function was copied from the homonymous function in `bevy_remote::builtin_methods`.
//       Remove once https://github.com/bevyengine/bevy/pull/22005 is merged and released.
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

/// Returns a [`ComponentMetadataMap`] entry
pub fn component_type_to_metadata<'metadata>(
    component_type: &str,
    metadata_map: &'metadata ComponentMetadataMap,
) -> Option<(ComponentId, &'metadata ComponentTypeMetadata)> {
    metadata_map.map.iter().find_map(|(id, meta)| {
        let full = meta.name.to_string();
        (full == component_type).then_some((*id, meta))
    })
}
