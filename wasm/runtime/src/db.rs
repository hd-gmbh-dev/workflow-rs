use js_sys::Object;
use js_sys::Uint8Array;
use rexie::ObjectStore;
use rexie::Rexie;
use rexie::TransactionMode;
use rkyv::de::deserializers::SharedDeserializeMapError;
use rkyv::ser::serializers::AllocSerializer;
use rkyv::ser::serializers::{
    AllocScratchError, CompositeSerializerError, SharedSerializeMapError,
};
use rkyv::ser::Serializer;
use rkyv::Deserialize;
use wasm_bindgen::prelude::*;
use wfrs_engine::state::State;
use wfrs_engine::state::WorkflowState;

pub struct DbEntry {
    pub id: String,
    pub state: WorkflowState,
    pub touched: f64,
}

impl DbEntry {
    pub fn new(id: String, state: WorkflowState) -> Self {
        Self {
            id,
            state,
            touched: js_sys::Date::new_0().get_time(),
        }
    }
}

pub struct IndexedDb {
    rexie: Rexie,
}

impl IndexedDb {
    pub async fn new() -> Result<Self, rexie::Error> {
        Ok(Self {
            rexie: Rexie::builder("wfrs")
                .version(1)
                .add_object_store(ObjectStore::new("instances").key_path("id"))
                .build()
                .await?,
        })
    }
}

async fn serialize_entry(
    entry: DbEntry,
) -> Result<
    Object,
    CompositeSerializerError<std::convert::Infallible, AllocScratchError, SharedSerializeMapError>,
> {
    let DbEntry { id, state, touched } = entry;
    let s = state.state().await;
    let mut serializer = AllocSerializer::<0>::default();
    serializer.serialize_value(&s.inner)?;
    let result = serializer.into_serializer().into_inner();
    let buf = js_sys::Uint8Array::new_with_length(result.len() as u32);
    buf.copy_from(&result);
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"id".into(), &id.into()).unwrap();
    js_sys::Reflect::set(&obj, &"data".into(), &buf.into()).unwrap();
    js_sys::Reflect::set(&obj, &"touched".into(), &touched.into()).unwrap();
    Ok(obj)
}

async fn store_entry(rexie: &Rexie, entry: JsValue) -> Result<(), rexie::Error> {
    let transaction = rexie.transaction(&["instances"], TransactionMode::ReadWrite)?;
    let instances = transaction.store("instances")?;
    instances.put(&entry, None).await?;
    transaction.done().await?;
    Ok(())
}

pub async fn deserialize_entry(data: &[u8]) -> Result<State, SharedDeserializeMapError> {
    let archived = unsafe { rkyv::archived_root::<State>(data) };
    archived.deserialize(&mut rkyv::de::deserializers::SharedDeserializeMap::default())
}

pub async fn read_entry(rexie: &Rexie, id: &str) -> Result<Option<Uint8Array>, rexie::Error> {
    let transaction = rexie.transaction(&["instances"], TransactionMode::ReadOnly)?;
    let instances = transaction.store("instances")?;
    let obj = instances.get(&id.into()).await?;
    if obj.is_object() {
        let entry = js_sys::Reflect::get(&obj, &"data".into()).unwrap();
        return Ok(Some(Uint8Array::unchecked_from_js(entry)));
    }
    Ok(None)
}

pub async fn remove_entry(rexie: &Rexie, id: &str) -> Result<(), rexie::Error> {
    let transaction = rexie.transaction(&["instances"], TransactionMode::ReadWrite)?;
    let instances = transaction.store("instances")?;
    instances.delete(&id.into()).await?;
    transaction.done().await?;
    Ok(())
}

impl IndexedDb {
    pub async fn store(&self, entry: DbEntry) -> anyhow::Result<()> {
        let entry: JsValue = serialize_entry(entry).await?.into();
        store_entry(&self.rexie, entry)
            .await
            .map_err(|e| anyhow::anyhow!("{e:#?}"))?;
        Ok(())
    }

    pub async fn get(&self, id: &str) -> anyhow::Result<Option<DbEntry>> {
        if let Some(state) = read_entry(&self.rexie, id)
            .await
            .map_err(|e| anyhow::anyhow!("{e:#?}"))?
        {
            let state = deserialize_entry(&state.to_vec()).await?;
            return Ok(Some(DbEntry {
                id: id.to_string(),
                state: WorkflowState::from_state(state),
                touched: 0.0,
            }));
        }
        Ok(None)
    }

    pub async fn remove(&self, id: &str) -> anyhow::Result<()> {
        remove_entry(&self.rexie, id)
            .await
            .map_err(|e| anyhow::anyhow!("{e:#?}"))
    }
}

impl AsRef<Rexie> for IndexedDb {
    fn as_ref(&self) -> &Rexie {
        &self.rexie
    }
}

async fn db() -> Result<IndexedDb, String> {
    IndexedDb::new().await.map_err(|err| format!("{err:#?}"))
}

pub async fn store(entry: DbEntry) -> Result<(), String> {
    db().await?
        .store(entry)
        .await
        .map_err(|err| format!("{err:#?}"))
}

pub async fn load(id: &str) -> Result<Option<DbEntry>, String> {
    db().await?.get(id).await.map_err(|err| format!("{err}"))
}

pub async fn remove(id: &str) -> Result<(), String> {
    db().await?
        .remove(id)
        .await
        .map_err(|err| format!("{err:#?}"))
}
