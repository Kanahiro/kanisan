#![deny(clippy::all)]

use std::any::TypeId;

use geozero::mvt::{tile, Message, Tile, TileValue};
use geozero::{geojson::GeoJson, wkb::Ewkb, GeozeroGeometry, ToGeo, ToMvt, ToWkb};
use napi::bindgen_prelude::{Array, Buffer, Either3, Float64Array, Object};
use napi::bindgen_prelude::{Either4, Either5};
use napi::{Either, JsBoolean, JsNull, JsNumber, JsObject, JsString, JsUnknown};

#[macro_use]
extern crate napi_derive;

fn add_feature_attribute(
  mvt_layer: &mut tile::Layer,
  mvt_feature: &mut tile::Feature,
  key: String,
  value: TileValue,
) {
  // https://github.com/georust/geozero/blob/main/geozero/src/mvt/mvt_writer.rs

  let mvt_value = value.into();
  let key_entry = mvt_layer.keys.iter().position(|k| *k == key);
  // Optimization: maintain a hash table with key/index pairs
  let key_idx = match key_entry {
    None => {
      mvt_layer.keys.push(key);
      mvt_layer.keys.len() - 1
    }
    Some(idx) => idx,
  };
  mvt_feature.tags.push(key_idx as u32);

  let val_entry = mvt_layer.values.iter().position(|v| *v == mvt_value);
  // Optimization: maintain a hash table with value/index pairs
  let validx = match val_entry {
    None => {
      mvt_layer.values.push(mvt_value);
      mvt_layer.values.len() - 1
    }
    Some(idx) => idx,
  };
  mvt_feature.tags.push(validx as u32);
}

#[napi]
pub fn generate_mvt(dataset: JsObject, extent: u32, bbox: Float64Array) -> Buffer {
  let layer_name: JsString = dataset.get("layer_name").unwrap().unwrap();
  let features: JsObject = dataset.get("features").unwrap().unwrap();
  let mut mvt_tile = Tile::default();
  let mut mvt_layer = tile::Layer {
    version: 2,
    ..Default::default()
  };
  mvt_layer.name = layer_name
    .into_utf8()
    .unwrap()
    .as_str()
    .unwrap()
    .to_string();
  mvt_layer.extent = Some(extent);

  Object::keys(&features).unwrap().iter().for_each(|key| {
    let feature: JsObject = features.get_named_property(key).unwrap();
    let wkb_buf: Buffer = feature.get_named_property("wkb_buf").unwrap();
    let properties: Object = feature.get_named_property("properties").unwrap();

    let ewkb = Ewkb(wkb_buf.to_vec());
    let mut mvt_feature: tile::Feature = ewkb
      .to_mvt(extent, bbox[0], bbox[1], bbox[2], bbox[3])
      .unwrap();
    mvt_feature.id = Some(key.parse::<u64>().unwrap());

    // Add properties
    Object::keys(&properties).unwrap().iter().for_each(|key| {
      let value: Either4<JsNumber, JsString, JsBoolean, JsNull> =
        properties.get_named_property(key).unwrap();
      let value = match value {
        Either4::A(value) => TileValue::Double(value.get_double().unwrap()),
        Either4::B(value) => {
          TileValue::Str(value.into_utf8().unwrap().as_str().unwrap().to_string())
        }
        Either4::C(value) => TileValue::Bool(value.get_value().unwrap()),
        _ => unimplemented!(),
      };

      add_feature_attribute(&mut mvt_layer, &mut mvt_feature, key.to_string(), value);
    });

    mvt_layer.features.push(mvt_feature);
  });

  mvt_tile.layers.push(mvt_layer);

  Buffer::from(mvt_tile.encode_to_vec())
}
