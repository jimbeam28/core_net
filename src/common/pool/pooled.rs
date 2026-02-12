// src/common/pool/pooled.rs
//
// Pooled object wrapper

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::ops::{Deref, DerefMut};

use super::{Pool, Clear};

/// Pooled object wrapper
///
/// Automatically returns object to pool when dropped
pub struct Pooled<T> {
    inner: Option<T>,
    pool: Option<Arc<Pool<T>>>,
    released: AtomicBool,
}

impl<T> Pooled<T> {
    pub fn new(item: T, pool: Arc<Pool<T>>) -> Self {
        Self {
            inner: Some(item),
            pool: Some(pool),
            released: AtomicBool::new(false),
        }
    }

    /// Get immutable reference to inner object
    pub fn inner(&self) -> &T {
        self.inner.as_ref().expect("object already released")
    }

    /// Get mutable reference to inner object
    pub fn inner_mut(&mut self) -> &mut T {
        self.inner.as_mut().expect("object already released")
    }

    /// Manually return object to pool (early release)
    pub fn release(mut self) {
        if let Some(item) = self.inner.take() {
            if let Some(pool) = self.pool.take() {
                pool.release(item);
            }
        }
    }

    /// Return and clear object
    pub fn release_and_clear(mut self)
    where
        T: Clear,
    {
        if let Some(mut item) = self.inner.take() {
            item.clear();
            if let Some(pool) = self.pool.take() {
                pool.release(item);
            }
        }
    }

    /// Detach inner object (consumes Pooled, does not return to pool)
    pub fn detach(mut self) -> T {
        self.inner.take().expect("inner object already released")
    }

    /// Transform inner object
    pub fn map<U, F>(self, f: F) -> U
    where
        F: FnOnce(T) -> U,
    {
        f(self.inner.expect("object already released"))
    }
}

impl<T> Deref for Pooled<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}

impl<T> DerefMut for Pooled<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner_mut()
    }
}

// Clone is intentionally not implemented to prevent accidental duplicate releases
