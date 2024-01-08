use std::sync::Arc;

use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

pub struct Locked<T>(pub Arc<RwLock<T>>);

impl<T> Locked<T> {
    pub fn new(inner: T) -> Self {
        Locked(Arc::new(RwLock::new(inner)))
    }

    pub async fn write(&self) -> RwLockWriteGuard<T> {
        self.0.write().await
    }

    pub async fn read(&self) -> RwLockReadGuard<T> {
        self.0.read().await
    }
}
