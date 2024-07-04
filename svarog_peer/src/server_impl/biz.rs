/// 模块职责:
/// 1. 将内部的两种Keystore和Signature转换为相同的gRPC消息格式; 以及反向转换.
/// 2. 为`keygen_mnem`, `reshare`这两个操作开辟线程. 这两个操作都有provider和consumer两个角色.
use std::collections::{BTreeMap, BTreeSet};

use erreur::*;
use svarog_grpc::{KeyTag, Mnemonic, SignTask, Signature};
use svarog_sesman::SvarogChannel;

use super::conversion::SignatureConversion;

pub(crate) async fn keygen_gg18(
    chan: SvarogChannel,
    player_name: String,
    i_dict: BTreeMap<usize, usize>,
    th_dict: BTreeMap<usize, usize>,
    players: BTreeMap<usize, BTreeSet<usize>>,
) -> Resultat<KeyTag> {
    use svarog_algo::elgamal_secp256k1::keygen;

    let key_id = chan.sid().to_owned();
    let keystore = keygen(chan, players, th_dict, i_dict, None)
        .await
        .catch_()?;
    let xpub = keystore.xpub().catch_()?;

    '_write: {
        use tokio::fs::{create_dir_all, write};
        create_dir_all("assets").await.catch_()?;
        let path = format!("assets/{}@{}.keystore", &player_name, &key_id);
        let keystore = serde_pickle::to_vec(&keystore, Default::default()).catch_()?;
        write(path, keystore).await.catch_()?;
    }
    Ok(KeyTag { key_id, xpub })
}

pub(crate) async fn keygen_frost(
    chan: SvarogChannel,
    player_name: String,
    i_dict: BTreeMap<usize, usize>,
    th_dict: BTreeMap<usize, usize>,
    players: BTreeMap<usize, BTreeSet<usize>>,
) -> Resultat<KeyTag> {
    use svarog_algo::schnorr_ed25519::keygen;

    let ctx = chan.sid().to_owned();
    let key_id = chan.sid().to_owned();
    let keystore = keygen(chan, players, th_dict, i_dict, None, ctx)
        .await
        .catch_()?;
    let xpub = keystore.xpub().catch_()?;

    '_write: {
        use tokio::fs::{create_dir_all, write};
        create_dir_all("assets").await.catch_()?;
        let path = format!("assets/{}@{}.keystore", &player_name, &key_id);
        let keystore = serde_pickle::to_vec(&keystore, Default::default()).catch_()?;
        write(path, keystore).await.catch_()?;
    }
    Ok(KeyTag { key_id, xpub })
}

pub(crate) async fn keygen_taproot(
    chan: SvarogChannel,
    player_name: String,
    i_dict: BTreeMap<usize, usize>,
    th_dict: BTreeMap<usize, usize>,
    players: BTreeMap<usize, BTreeSet<usize>>,
) -> Resultat<KeyTag> {
    use svarog_algo::schnorr_secp256k1::keygen;

    let ctx = chan.sid().to_owned();
    let key_id = chan.sid().to_owned();
    let keystore = keygen(chan, players, th_dict, i_dict, None, ctx)
        .await
        .catch_()?;
    let xpub = keystore.xpub().catch_()?;

    '_write: {
        use tokio::fs::{create_dir_all, write};
        create_dir_all("assets").await.catch_()?;
        let path = format!("assets/{}@{}.keystore", &player_name, &key_id);
        let keystore = serde_pickle::to_vec(&keystore, Default::default()).catch_()?;
        write(path, keystore).await.catch_()?;
    }
    Ok(KeyTag { key_id, xpub })
}

pub(crate) async fn keygen_mnem_gg18(
    chan: SvarogChannel,
    player_name: String,
    i_dict: BTreeMap<usize, usize>,
    th_dict: BTreeMap<usize, usize>,
    players: BTreeMap<usize, BTreeSet<usize>>,
    mnem: Option<Mnemonic>,
) -> Resultat<KeyTag> {
    use svarog_algo::elgamal_secp256k1::{keygen_mnem_consumer, keygen_mnem_provider};

    let provider_thread = if let Some(mnem) = mnem {
        let future: _ =
            keygen_mnem_provider(chan.clone(), players.clone(), mnem.words, mnem.password);
        let handle: _ = tokio::spawn(future);
        Some(handle)
    } else {
        None
    };

    let key_id = chan.sid().to_owned();
    let ret = if false == i_dict.is_empty() {
        let keystore = keygen_mnem_consumer(chan, players, th_dict, i_dict)
            .await
            .catch_()?;
        let xpub = keystore.xpub().catch_()?;

        '_write: {
            use tokio::fs::{create_dir_all, write};
            create_dir_all("assets").await.catch_()?;
            let path = format!("assets/{}@{}.keystore", &player_name, &key_id);
            let keystore = serde_pickle::to_vec(&keystore, Default::default()).catch_()?;
            write(path, keystore).await.catch_()?;
        }
        KeyTag { key_id, xpub }
    } else {
        KeyTag {
            key_id,
            xpub: "".to_owned(),
        }
    };

    if let Some(handle) = provider_thread {
        handle
            .await
            .catch("ThreadFailed", "due to panic")?
            .catch("ThreadFailed", "due to exception")?;
    }

    Ok(ret)
}

pub(crate) async fn keygen_mnem_frost(
    chan: SvarogChannel,
    player_name: String,
    i_dict: BTreeMap<usize, usize>,
    th_dict: BTreeMap<usize, usize>,
    players: BTreeMap<usize, BTreeSet<usize>>,
    mnem: Option<Mnemonic>,
) -> Resultat<KeyTag> {
    use svarog_algo::schnorr_ed25519::{keygen_mnem_consumer, keygen_mnem_provider};

    let provider_thread = if let Some(mnem) = mnem {
        let future: _ =
            keygen_mnem_provider(chan.clone(), players.clone(), mnem.words, mnem.password);
        let handle: _ = tokio::spawn(future);
        Some(handle)
    } else {
        None
    };

    let ctx = chan.sid().to_owned();
    let key_id = chan.sid().to_owned();

    let ret = if false == i_dict.is_empty() {
        let keystore = keygen_mnem_consumer(chan, players, th_dict, i_dict, ctx)
            .await
            .catch_()?;
        let xpub = keystore.xpub().catch_()?;

        '_write: {
            use tokio::fs::{create_dir_all, write};
            create_dir_all("assets").await.catch_()?;
            let path = format!("assets/{}@{}.keystore", &player_name, &key_id);
            let keystore = serde_pickle::to_vec(&keystore, Default::default()).catch_()?;
            write(path, keystore).await.catch_()?;
        }
        KeyTag { key_id, xpub }
    } else {
        KeyTag {
            key_id,
            xpub: "".to_owned(),
        }
    };

    if let Some(handle) = provider_thread {
        handle
            .await
            .catch("ThreadFailed", "due to panic")?
            .catch("ThreadFailed", "due to exception")?;
    }

    Ok(ret)
}

pub(crate) async fn keygen_mnem_taproot(
    chan: SvarogChannel,
    player_name: String,
    i_dict: BTreeMap<usize, usize>,
    th_dict: BTreeMap<usize, usize>,
    players: BTreeMap<usize, BTreeSet<usize>>,
    mnem: Option<Mnemonic>,
) -> Resultat<KeyTag> {
    use svarog_algo::schnorr_secp256k1::{keygen_mnem_consumer, keygen_mnem_provider};

    let provider_thread = if let Some(mnem) = mnem {
        let future: _ =
            keygen_mnem_provider(chan.clone(), players.clone(), mnem.words, mnem.password);
        let handle: _ = tokio::spawn(future);
        Some(handle)
    } else {
        None
    };

    let ctx = chan.sid().to_owned();
    let key_id = chan.sid().to_owned();

    let ret = if false == i_dict.is_empty() {
        let keystore = keygen_mnem_consumer(chan, players, th_dict, i_dict, ctx)
            .await
            .catch_()?;
        let xpub = keystore.xpub().catch_()?;

        '_write: {
            use tokio::fs::{create_dir_all, write};
            create_dir_all("assets").await.catch_()?;
            let path = format!("assets/{}@{}.keystore", &player_name, &key_id);
            let keystore = serde_pickle::to_vec(&keystore, Default::default()).catch_()?;
            write(path, keystore).await.catch_()?;
        }
        KeyTag { key_id, xpub }
    } else {
        KeyTag {
            key_id,
            xpub: "".to_owned(),
        }
    };

    if let Some(handle) = provider_thread {
        handle
            .await
            .catch("ThreadFailed", "due to panic")?
            .catch("ThreadFailed", "due to exception")?;
    }

    Ok(ret)
}

pub(crate) async fn sign_gg18(
    chan: SvarogChannel,
    player_name: String,
    key_id: String,
    signers: BTreeMap<usize, BTreeSet<usize>>,
    tasks: Vec<SignTask>,
) -> Resultat<Vec<Signature>> {
    use svarog_algo::elgamal_secp256k1::{sign_batch, KeystoreElgamal};

    assert_throw!(tasks.len() >= 1);
    assert_throw!(signers.len() >= 1);
    for (_, dept_signers) in signers.iter() {
        assert_throw!(dept_signers.len() >= 1);
    }
    let keystore: KeystoreElgamal = {
        use tokio::fs::read;
        let path = format!("assets/{}@{}.keystore", &player_name, &key_id);
        let keystore = read(path).await.catch_()?;
        let mut keystore: KeystoreElgamal =
            serde_pickle::from_slice(&keystore, Default::default()).catch_()?;
        keystore.paillier_key.precompute_cache().catch_()?;
        keystore
    };
    for (i, dept_i) in keystore.i_dict.iter() {
        let dept_signers = signers.get(dept_i).ifnone_()?;
        assert_throw!(dept_signers.contains(i));
    }

    let res = {
        let tasks = tasks
            .into_iter()
            .map(|task| (task.msg, task.dpath))
            .collect();
        let sigs = sign_batch(chan, signers, keystore, tasks).await.catch_()?;
        let mut res = Vec::new();
        for sig in sigs.into_iter() {
            let sig = sig.to_proto().catch_()?;
            res.push(sig);
        }
        res
    };
    Ok(res)
}

pub(crate) async fn sign_frost(
    chan: SvarogChannel,
    player_name: String,
    key_id: String,
    signers: BTreeMap<usize, BTreeSet<usize>>,
    tasks: Vec<SignTask>,
) -> Resultat<Vec<Signature>> {
    use svarog_algo::schnorr_ed25519::{sign_batch, KeystoreSchnorr};

    assert_throw!(tasks.len() >= 1);
    assert_throw!(signers.len() >= 1);
    for (_, dept_signers) in signers.iter() {
        assert_throw!(dept_signers.len() >= 1);
    }
    let keystore: KeystoreSchnorr = {
        use tokio::fs::read;
        let path = format!("assets/{}@{}.keystore", &player_name, &key_id);
        let keystore = read(path).await.catch_()?;
        let keystore = serde_pickle::from_slice(&keystore, Default::default()).catch_()?;
        keystore
    };
    for (i, dept_i) in keystore.i_dict.iter() {
        let dept_signers = signers.get(dept_i).ifnone_()?;
        assert_throw!(dept_signers.contains(i));
    }

    let res = {
        let tasks = tasks
            .into_iter()
            .map(|task| (task.msg, task.dpath))
            .collect();
        let sigs = sign_batch(chan, signers, keystore, tasks).await.catch_()?;
        let mut res = Vec::new();
        for sig in sigs.into_iter() {
            let sig = sig.to_proto().catch_()?;
            res.push(sig);
        }
        res
    };
    Ok(res)
}

pub(crate) async fn sign_taproot(
    chan: SvarogChannel,
    player_name: String,
    key_id: String,
    signers: BTreeMap<usize, BTreeSet<usize>>,
    tasks: Vec<SignTask>,
) -> Resultat<Vec<Signature>> {
    use svarog_algo::schnorr_secp256k1::{sign_batch, KeystoreSchnorr};

    assert_throw!(tasks.len() >= 1);
    assert_throw!(signers.len() >= 1);
    for (_, dept_signers) in signers.iter() {
        assert_throw!(dept_signers.len() >= 1);
    }
    let keystore: KeystoreSchnorr = {
        use tokio::fs::read;
        let path = format!("assets/{}@{}.keystore", &player_name, &key_id);
        let keystore = read(path).await.catch_()?;
        let keystore = serde_pickle::from_slice(&keystore, Default::default()).catch_()?;
        keystore
    };
    for (i, dept_i) in keystore.i_dict.iter() {
        let dept_signers = signers.get(dept_i).ifnone_()?;
        assert_throw!(dept_signers.contains(i));
    }

    let res = {
        let tasks = {
            let mut tasks1 = Vec::new();
            for task in tasks.into_iter() {
                let msg = task.msg;
                let dpath = task.dpath;
                let mkroot = task.mkroot;
                tasks1.push((msg, dpath, mkroot));
            }
            tasks1
        };
        let sigs = sign_batch(chan, signers, keystore, tasks).await.catch_()?;
        let mut res = Vec::new();
        for sig in sigs.into_iter() {
            let sig = sig.to_proto().catch_()?;
            res.push(sig);
        }
        res
    };
    Ok(res)
}

pub(crate) async fn reshare_gg18(
    chan: SvarogChannel,
    player_name: String,
    key_id: String,
    i_dict_reshared: BTreeMap<usize, usize>,
    th_dict_reshared: BTreeMap<usize, usize>,
    providers: BTreeMap<usize, BTreeSet<usize>>,
    consumers: BTreeMap<usize, BTreeSet<usize>>,
) -> Resultat<KeyTag> {
    use svarog_algo::elgamal_secp256k1::{reshare_consumer, reshare_provider, KeystoreElgamal};

    assert_throw!(providers.len() >= 1);
    for (_, dept_providers) in providers.iter() {
        assert_throw!(dept_providers.len() >= 1);
    }
    assert_throw!(consumers.len() >= 1);
    for (_, dept_consumers) in consumers.iter() {
        assert_throw!(dept_consumers.len() >= 1);
    }
    let keystore: Option<KeystoreElgamal> = if false == key_id.is_empty() {
        use tokio::fs::read;

        let path = format!("assets/{}@{}.keystore", &player_name, &key_id);
        let keystore = read(path).await.catch_()?;
        let keystore = serde_pickle::from_slice(&keystore, Default::default()).catch_()?;
        Some(keystore)
    } else {
        None
    };

    for (i, dept_i) in i_dict_reshared.iter() {
        let dept_consumers = consumers.get(dept_i).ifnone_()?;
        assert_throw!(dept_consumers.contains(i));
    }

    let provider_thread = if let Some(keystore) = keystore {
        let future: _ =
            reshare_provider(chan.clone(), keystore, providers.clone(), consumers.clone());
        let handle: _ = tokio::spawn(future);
        Some(handle)
    } else {
        None
    };

    let key_id = chan.sid().to_owned();
    let ret = if false == i_dict_reshared.is_empty() {
        let keystore = reshare_consumer(
            chan,
            th_dict_reshared,
            i_dict_reshared,
            providers,
            consumers,
        )
        .await
        .catch_()?;
        let xpub = keystore.xpub().catch_()?;

        '_write: {
            use tokio::fs::{create_dir_all, write};
            create_dir_all("assets").await.catch_()?;
            let path = format!("assets/{}@{}.keystore", &player_name, &key_id);
            let keystore = serde_pickle::to_vec(&keystore, Default::default()).catch_()?;
            write(path, keystore).await.catch_()?;
        }
        KeyTag { key_id, xpub }
    } else {
        KeyTag {
            key_id,
            xpub: "".to_owned(),
        }
    };

    if let Some(handle) = provider_thread {
        handle
            .await
            .catch("ThreadFailed", "due to panic")?
            .catch("ThreadFailed", "due to exception")?;
    }

    Ok(ret)
}

pub(crate) async fn reshare_frost(
    chan: SvarogChannel,
    player_name: String,
    key_id: String,
    i_dict_reshared: BTreeMap<usize, usize>,
    th_dict_reshared: BTreeMap<usize, usize>,
    providers: BTreeMap<usize, BTreeSet<usize>>,
    consumers: BTreeMap<usize, BTreeSet<usize>>,
) -> Resultat<KeyTag> {
    use svarog_algo::schnorr_ed25519::{reshare_consumer, reshare_provider, KeystoreSchnorr};

    assert_throw!(providers.len() >= 1);
    for (_, dept_providers) in providers.iter() {
        assert_throw!(dept_providers.len() >= 1);
    }
    assert_throw!(consumers.len() >= 1);
    for (_, dept_consumers) in consumers.iter() {
        assert_throw!(dept_consumers.len() >= 1);
    }
    let keystore: Option<KeystoreSchnorr> = if false == key_id.is_empty() {
        use tokio::fs::read;

        let path = format!("assets/{}@{}.keystore", &player_name, &key_id);
        let keystore = read(path).await.catch_()?;
        let keystore = serde_pickle::from_slice(&keystore, Default::default()).catch_()?;
        Some(keystore)
    } else {
        None
    };
    for (i, dept_i) in i_dict_reshared.iter() {
        let dept_consumers = consumers.get(dept_i).ifnone_()?;
        assert_throw!(dept_consumers.contains(i));
    }

    let provider_thread = if let Some(keystore) = keystore {
        let future: _ =
            reshare_provider(chan.clone(), keystore, providers.clone(), consumers.clone());
        let handle: _ = tokio::spawn(future);
        Some(handle)
    } else {
        None
    };

    let key_id = chan.sid().to_owned();
    let ret = if false == i_dict_reshared.is_empty() {
        let ctx = chan.sid().to_owned();
        let keystore = reshare_consumer(
            chan,
            th_dict_reshared,
            i_dict_reshared,
            providers,
            consumers,
            ctx,
        )
        .await
        .catch_()?;
        let xpub = keystore.xpub().catch_()?;

        '_write: {
            use tokio::fs::{create_dir_all, write};
            create_dir_all("assets").await.catch_()?;
            let path = format!("assets/{}@{}.keystore", &player_name, &key_id);
            let keystore = serde_pickle::to_vec(&keystore, Default::default()).catch_()?;
            write(path, keystore).await.catch_()?;
        }
        KeyTag { key_id, xpub }
    } else {
        KeyTag {
            key_id,
            xpub: "".to_owned(),
        }
    };

    if let Some(handle) = provider_thread {
        handle
            .await
            .catch("ThreadFailed", "due to panic")?
            .catch("ThreadFailed", "due to exception")?;
    }

    Ok(ret)
}

pub(crate) async fn reshare_taproot(
    chan: SvarogChannel,
    player_name: String,
    key_id: String,
    i_dict_reshared: BTreeMap<usize, usize>,
    th_dict_reshared: BTreeMap<usize, usize>,
    providers: BTreeMap<usize, BTreeSet<usize>>,
    consumers: BTreeMap<usize, BTreeSet<usize>>,
) -> Resultat<KeyTag> {
    use svarog_algo::schnorr_secp256k1::{reshare_consumer, reshare_provider, KeystoreSchnorr};

    assert_throw!(providers.len() >= 1);
    for (_, dept_providers) in providers.iter() {
        assert_throw!(dept_providers.len() >= 1);
    }
    assert_throw!(consumers.len() >= 1);
    for (_, dept_consumers) in consumers.iter() {
        assert_throw!(dept_consumers.len() >= 1);
    }
    let keystore: Option<KeystoreSchnorr> = if false == key_id.is_empty() {
        use tokio::fs::read;

        let path = format!("assets/{}@{}.keystore", &player_name, &key_id);
        let keystore = read(path).await.catch_()?;
        let keystore = serde_pickle::from_slice(&keystore, Default::default()).catch_()?;
        Some(keystore)
    } else {
        None
    };
    for (i, dept_i) in i_dict_reshared.iter() {
        let dept_consumers = consumers.get(dept_i).ifnone_()?;
        assert_throw!(dept_consumers.contains(i));
    }

    let provider_thread = if let Some(keystore) = keystore {
        let future: _ =
            reshare_provider(chan.clone(), keystore, providers.clone(), consumers.clone());
        let handle: _ = tokio::spawn(future);
        Some(handle)
    } else {
        None
    };

    let key_id = chan.sid().to_owned();
    let ret = if false == i_dict_reshared.is_empty() {
        let ctx = chan.sid().to_owned();
        let keystore = reshare_consumer(
            chan,
            th_dict_reshared,
            i_dict_reshared,
            providers,
            consumers,
            ctx,
        )
        .await
        .catch_()?;
        let xpub = keystore.xpub().catch_()?;

        '_write: {
            use tokio::fs::{create_dir_all, write};
            create_dir_all("assets").await.catch_()?;
            let path = format!("assets/{}@{}.keystore", &player_name, &key_id);
            let keystore = serde_pickle::to_vec(&keystore, Default::default()).catch_()?;
            write(path, keystore).await.catch_()?;
        }
        KeyTag { key_id, xpub }
    } else {
        KeyTag {
            key_id,
            xpub: "".to_owned(),
        }
    };

    if let Some(handle) = provider_thread {
        handle
            .await
            .catch("ThreadFailed", "due to panic")?
            .catch("ThreadFailed", "due to exception")?;
    }

    Ok(ret)
}
