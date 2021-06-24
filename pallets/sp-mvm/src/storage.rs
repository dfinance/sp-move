use core::marker::PhantomData;
use sp_std::prelude::*;
use codec::FullCodec;
use codec::FullEncode;
use frame_support::storage::StorageMap;
use move_vm::io::traits::Storage;

pub trait MoveVmStorage<T, K: FullEncode, V: FullCodec> {
    type VmStorage;

    fn move_vm_storage() -> StorageAdapter<Self::VmStorage, K, V>
    where
        Self::VmStorage: StorageMap<K, V, Query = Option<V>>,
    {
        Default::default()
    }
}

/// Vm storage adapter for native storage
pub struct StorageAdapter<T, K = Vec<u8>, V = Vec<u8>>(PhantomData<(T, K, V)>);

impl<T, K, V> Default for StorageAdapter<T, K, V> {
    fn default() -> Self {
        Self(Default::default())
    }
}

/// Default VM storage implementation
impl<T: StorageMap<Vec<u8>, Vec<u8>, Query = Option<Vec<u8>>>> Storage
    for StorageAdapter<T, Vec<u8>, Vec<u8>>
{
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        trace!("storage::get {:?}", key);
        T::get(key)
    }

    fn insert(&self, key: &[u8], value: &[u8]) {
        trace!("storage::set {:?} <= {} bytes", key, value.len());
        T::insert(key, value)
    }

    fn remove(&self, key: &[u8]) {
        trace!("storage::rem {:?}", key);
        T::remove(key)
    }
}

#[cfg(not(feature = "no-vm-static"))]
pub mod boxed {
    use sp_std::prelude::*;
    pub type VmStorageAdapter = VmStorageBoxAdapter;

    /// Vm storage boxed adapter for native storage
    pub struct VmStorageBoxAdapter {
        f_get: Box<dyn Fn(&[u8]) -> Option<Vec<u8>>>,
        f_insert: Box<dyn Fn(&[u8], &[u8])>,
        f_remove: Box<dyn Fn(&[u8])>,
    }

    pub fn into_boxfn_adapter<T>() -> VmStorageBoxAdapter
    where
        T: super::StorageMap<Vec<u8>, Vec<u8>, Query = Option<Vec<u8>>>,
    {
        #![allow(clippy::redundant_closure)]
        VmStorageBoxAdapter {
            f_get: Box::new(|key: &[u8]| T::get(key)),
            f_insert: Box::new(|key, value| T::insert(key, value)),
            f_remove: Box::new(|key| T::remove(key)),
        }
    }

    impl<T> From<super::StorageAdapter<T, Vec<u8>, Vec<u8>>> for VmStorageBoxAdapter
    where
        T: super::StorageMap<Vec<u8>, Vec<u8>, Query = Option<Vec<u8>>>,
    {
        fn from(_: super::StorageAdapter<T, Vec<u8>, Vec<u8>>) -> Self {
            into_boxfn_adapter::<T>()
        }
    }

    impl move_vm::io::traits::Storage for VmStorageBoxAdapter {
        fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
            trace!("storage::get {:?}", key);
            (self.f_get)(key)
        }

        fn insert(&self, key: &[u8], value: &[u8]) {
            trace!("storage::set {:?} <= {} bytes", key, value.len());
            (self.f_insert)(key, value)
        }

        fn remove(&self, key: &[u8]) {
            trace!("storage::rem {:?}", key);
            (self.f_remove)(key)
        }
    }
}
