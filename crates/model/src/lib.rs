use rkyv::ser::serializers::AllocSerializer;
use rkyv::ser::Serializer;
use rkyv::{AlignedVec, Deserialize};

pub mod jsep;
pub mod json;
mod model;
pub use model::*;

pub fn serialize(workflow: WorkflowDefinition) -> AlignedVec {
    let mut serializer = AllocSerializer::<0>::default();
    serializer.serialize_value(&workflow).unwrap();
    serializer.into_serializer().into_inner()
}

pub fn deserialize(
    data: &[u8],
) -> Result<WorkflowDefinition, rkyv::de::deserializers::SharedDeserializeMapError> {
    let archived = unsafe { rkyv::archived_root::<WorkflowDefinition>(data) };
    archived.deserialize(&mut rkyv::de::deserializers::SharedDeserializeMap::default())
}
