// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the "hack" directory of this source tree.

use dashmap::DashMap;
use hackrs::cache::Cache;
use serde::{de::DeserializeOwned, Serialize};
use std::cmp::Eq;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

pub struct SerializingCache<K: Hash + Eq, V: Serialize + DeserializeOwned> {
    cache: DashMap<K, Box<[u8]>>,
    compression: Compression,
    _phantom: PhantomData<V>,
}

#[derive(Copy, Clone, Debug)]
pub enum Compression {
    None,
    Lz4,
}

impl<K, V> Default for SerializingCache<K, V>
where
    K: Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    fn default() -> Self {
        Self {
            cache: Default::default(),
            compression: Default::default(),
            _phantom: PhantomData,
        }
    }
}

impl Default for Compression {
    fn default() -> Self {
        Self::Lz4
    }
}

impl<K, V> SerializingCache<K, V>
where
    K: Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_compression(compression: Compression) -> Self {
        Self {
            cache: Default::default(),
            compression,
            _phantom: PhantomData,
        }
    }
}

impl<K, V> Cache<K, V> for SerializingCache<K, V>
where
    K: Copy + Send + Sync + Hash + Eq,
    V: Clone + Send + Sync + Serialize + DeserializeOwned,
{
    fn get(&self, key: K) -> Option<V> {
        self.cache.get(&key).map(|val| match self.compression {
            Compression::None => deserialize(&val),
            Compression::Lz4 => {
                let serialized = lz4_decompress(&val);
                deserialize(&serialized)
            }
        })
    }

    fn insert(&self, key: K, val: V) {
        let val = serialize(&val);
        let val = match self.compression {
            Compression::None => val,
            Compression::Lz4 => lz4_compress(&val),
        };
        self.cache.insert(key, val.into_boxed_slice());
    }
}

fn serialize<T: Serialize>(val: &T) -> Vec<u8> {
    let mut serialized = Vec::new();
    let _guard = intern::SerGuard::default();
    bincode::serialize_into(&mut serialized, val).unwrap();
    serialized
}

fn deserialize<T: DeserializeOwned>(serialized: &[u8]) -> T {
    let _guard = intern::DeGuard::default();
    bincode::deserialize(serialized).unwrap()
}

fn lz4_compress(mut bytes: &[u8]) -> Vec<u8> {
    let mut encoder = lz4::EncoderBuilder::new().level(1).build(vec![]).unwrap();
    std::io::copy(&mut bytes, &mut encoder).unwrap();
    let (compressed, result) = encoder.finish();
    result.unwrap();
    compressed
}

fn lz4_decompress(compressed: &[u8]) -> Vec<u8> {
    let mut decompressed = vec![];
    let mut decoder = lz4::Decoder::new(compressed).unwrap();
    std::io::copy(&mut decoder, &mut decompressed).unwrap();
    decompressed
}

impl<K, V> Debug for SerializingCache<K, V>
where
    K: Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SerializingCache").finish()
    }
}
