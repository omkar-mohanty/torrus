use torrus_core::store::Store;

pub struct Storage<S: Store> {
    pub store: S,
}
