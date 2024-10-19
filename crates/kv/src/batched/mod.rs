pub mod batched_index;
pub mod batched_write;
pub mod log_record;

#[cfg(test)]
mod tests {
    use std::fs;

    use bytes::Bytes;

    use crate::{
        config::config::Config,
        store::{store::Store, utils::TempStore},
    };

    #[test]
    fn test_batched() {
        let (_raii, store) = TempStore::init(0);

        let batch1 = store.new_batched();
        for i in [0, 1, 2] {
            let key = format!("{}", i);
            let val = english_numbers::convert_all_fmt(i);
            batch1.put(key.into(), val.into()).unwrap();
        }
        let batch2 = store.new_batched();
        for i in [10, 11, 12] {
            let key = format!("{}", i);
            let val = english_numbers::convert_all_fmt(i);
            batch2.put(key.into(), val.into()).unwrap();
        }

        batch1.commit().unwrap();
        assert_eq!(
            store.list_keys(),
            [
                Bytes::from(b"0".to_vec()),
                Bytes::from(b"1".to_vec()),
                Bytes::from(b"2".to_vec())
            ]
        );

        store
            .put(b"114514".to_vec().into(), b"Bad Number".to_vec().into())
            .unwrap();
        assert_eq!(
            store.list_keys(),
            [
                Bytes::from(b"0".to_vec()),
                Bytes::from(b"1".to_vec()),
                Bytes::from(b"114514".to_vec()),
                Bytes::from(b"2".to_vec())
            ]
        );

        batch2.commit().unwrap();
        assert_eq!(
            store.list_keys(),
            [
                Bytes::from(b"0".to_vec()),
                Bytes::from(b"1".to_vec()),
                Bytes::from(b"10".to_vec()),
                Bytes::from(b"11".to_vec()),
                Bytes::from(b"114514".to_vec()),
                Bytes::from(b"12".to_vec()),
                Bytes::from(b"2".to_vec()),
            ]
        );
    }

    #[test]
    fn test_persistence() {
        let test_id = 7;
        let dir = format!("store/test_{}", test_id);

        {
            // remove if exist
            fs::remove_dir_all(dir.clone());
            let (mut store_config, file_config, batched_config) =
                Config::from_toml("config.toml".into());
            store_config.dir = dir.clone().into();
            let store = Store::open(store_config, file_config, batched_config).unwrap();

            let batch1 = store.new_batched();
            let batch2 = store.new_batched();
            let batch3 = store.new_batched();
            for i in 0..100 {
                let key1 = format!("{}", i);
                let val1 = english_numbers::convert_all_fmt(i);
                let key2 = format!("{}", i + 100);
                let val2 = english_numbers::convert_all_fmt(i + 100);
                let key3 = format!("{}", i + 200);
                let val3 = english_numbers::convert_all_fmt(i + 200);
                batch1.put(key1.into(), val1.into()).unwrap();
                batch2.put(key2.into(), val2.into()).unwrap();
                batch3.put(key3.into(), val3.into()).unwrap();
            }
            batch1.commit().unwrap();
            batch2.commit().unwrap();
            batch3.commit().unwrap();
            for _ in 0..10 {
                let batch = store.new_batched();
                batch.commit().unwrap();
            }

            // do not clean up
        }

        {
            let (mut store_config, file_config, batched_config) =
                Config::from_toml("config.toml".into());
            store_config.dir = dir.clone().into();
            let store = Store::open(store_config, file_config, batched_config).unwrap();

            assert_eq!(
                store.get("123".into()).unwrap(),
                Bytes::from("One Hundred and Twenty-Three")
            );
            assert_eq!(
                store.get("246".into()).unwrap(),
                Bytes::from("Two Hundred and Forty-Six")
            );
            assert_eq!(store.get("53".into()).unwrap(), Bytes::from("Fifty-Three"));
            assert_eq!(store.list_keys().len(), 300);
            assert_eq!(
                store.batch_id.load(std::sync::atomic::Ordering::Relaxed),
                13
            );

            let batch = store.new_batched();
            for i in 0..100 {
                let key = format!("{}", i);
                let val = english_numbers::convert_all_fmt(i);
                batch.put(key.into(), val.into()).unwrap();
            }
            assert_eq!(
                store.batch_id.load(std::sync::atomic::Ordering::Relaxed),
                13
            );
            batch.commit().unwrap();
            assert_eq!(
                store.batch_id.load(std::sync::atomic::Ordering::Relaxed),
                14
            );
        }
    }

    #[test]
    fn test_batched_overwrite() {
        let (_raii, store) = TempStore::init(2);

        let batch = store.new_batched();
        for i in [0, 1, 2] {
            let key = format!("{}", i);
            let val = english_numbers::convert_all_fmt(i);
            batch.put(key.into(), val.into()).unwrap();
        }

        store
            .put(b"1".to_vec().into(), b"Yi".to_vec().into())
            .unwrap();
        store
            .put(b"2".to_vec().into(), b"Er".to_vec().into())
            .unwrap();
        store
            .put(b"3".to_vec().into(), b"San".to_vec().into())
            .unwrap();

        for kv in store.iter_options().make() {
            dbg!(kv);
        }

        batch.commit().unwrap();
        for kv in store.iter_options().make() {
            dbg!(kv);
        }

        assert_eq!(store.batch_id.load(std::sync::atomic::Ordering::Relaxed), 1);
        let batch1 = store.new_batched();
        let batch2 = store.new_batched();
        assert_eq!(store.batch_id.load(std::sync::atomic::Ordering::Relaxed), 1);
        batch1.commit().unwrap();
        batch2.commit().unwrap();
        assert_eq!(store.batch_id.load(std::sync::atomic::Ordering::Relaxed), 3);
    }
}

#[cfg(test)]
/// This shall be run on specific order with specific commands
mod test_truncations {

    use std::fs;

    use crate::{
        config::config::Config,
        store::{store::Store, utils::TempStore},
    };

    #[test]
    // First, interrupt while running this.
    fn test_truncated() {
        std::env::set_var("RUST_LOG", "trace");
        pretty_env_logger::init();

        let test_id = 6;
        let dir = format!("store/test_{}", test_id);

        // remove if exist
        fs::remove_dir_all(dir.clone());
        let (mut store_config, file_config, mut batched_config) =
            Config::from_toml("config.toml".into());
        batched_config.max_batch_size = 100000;

        store_config.dir = dir.clone().into();
        let store = Store::open(store_config, file_config, batched_config).unwrap();

        let batch = store.new_batched();

        for i in 0..100000 {
            let key = format!("{}", i);
            let val = english_numbers::convert_all_fmt(i);
            batch.put(key.into(), val.into()).unwrap();
            if (i + 1) % 1000 == 0 {
                info!("Storing i = {}", i);
            }
        }
    }

    /* Then, run the following two. in order. */

    #[test]
    fn test_continue() {
        std::env::set_var("RUST_LOG", "trace");
        pretty_env_logger::init();

        let test_id = 6;
        let dir = format!("store/test_{}", test_id);

        let (mut store_config, file_config, mut batched_config) =
            Config::from_toml("config.toml".into());
        batched_config.max_batch_size = 100000;

        store_config.dir = dir.clone().into();
        let store = Store::open(store_config, file_config, batched_config).unwrap();

        store.put("hello".into(), "world".into()).unwrap();
        store.put("goodbye".into(), "world".into()).unwrap();

        let batch = store.new_batched();
        for i in 0..100 {
            let key = format!("hello{}", i);
            let val = format!("world{}", i);
            batch.put(key.into(), val.into()).unwrap();
        }
        batch.commit().unwrap();
    }

    #[test]
    fn test_read_truncated() {
        let test_id = 6;
        let dir = format!("store/test_{}", test_id);

        let (mut store_config, file_config, mut batched_config) =
            Config::from_toml("config.toml".into());
        batched_config.max_batch_size = 100000;
        store_config.dir = dir.clone().into();
        let store = Store::open(store_config, file_config, batched_config).unwrap();

        dbg!(store.list_keys());
        dbg!(&store.batch_id);

        // assert_eq!(store.list_keys().len(), 0);
        // assert_ne!(
        //     store
        //         .active_file_id
        //         .load(std::sync::atomic::Ordering::Relaxed),
        //     0
        // );
    }
}
