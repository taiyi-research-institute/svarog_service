use std::collections::BTreeSet;

use erreur::*;
use svarog_algo::{
    elgamal_secp256k1::{
        keygen, keygen_mnem_consumer, keygen_mnem_provider, reshare_consumer, reshare_provider,
        sign_batch, ImportedPartyKey, KeystoreElgamal,
    },
    mnemi2sk,
};
use svarog_sesman::SvarogChannel;

use crate::{
    ses_arch,
    structs::{Mnemonics, SignTask, Signature},
};

pub async fn biz_keygen(
    sesman_url: String,
    session_id: String,
    member_name: String,
) -> Resultat<KeystoreElgamal> {
    assert_throw!(sesman_url.starts_with("http://") || sesman_url.starts_with("https://"));
    let https = sesman_url.starts_with("https://");

    let (chan, cfg) = SvarogChannel::use_session(&session_id, &sesman_url, https)
        .await
        .catch_()?;
    let t = cfg.threshold as usize;
    let (i, players) = ses_arch(&member_name, &cfg.players);
    assert_throw!(
        players.len() == cfg.players.len(),
        "all keygen members should attend"
    );
    let keystore = impl_keygen(chan, i, t, players).await.catch_()?;
    Ok(keystore)
}

pub async fn biz_keygen_mnem(
    sesman_url: String,
    session_id: String,
    member_name: String,
    mnemonics: Option<Mnemonics>,
) -> Resultat<Option<KeystoreElgamal>> {
    assert_throw!(sesman_url.starts_with("http://") || sesman_url.starts_with("https://"));
    let https = sesman_url.starts_with("https://");

    let (chan, cfg) = SvarogChannel::use_session(&session_id, &sesman_url, https)
        .await
        .catch_()?;
    let t = cfg.threshold as usize;
    let (i, players) = ses_arch(&member_name, &cfg.players);
    assert_throw!(
        players.len() == cfg.players.len(),
        "all keygen members should attend"
    );
    let keystore = impl_keygen_mnem(chan, i, t, players, mnemonics)
        .await
        .catch_()?;
    Ok(keystore)
}

pub async fn biz_keygen_mnemi(
    sesman_url: String,
    session_id: String,
    member_name: String,
    mnem_i: String,
) -> Resultat<KeystoreElgamal> {
    assert_throw!(sesman_url.starts_with("http://") || sesman_url.starts_with("https://"));
    let https = sesman_url.starts_with("https://");

    let (chan, cfg) = SvarogChannel::use_session(&session_id, &sesman_url, https)
        .await
        .catch_()?;
    let t = cfg.threshold as usize;
    let (i, players) = ses_arch(&member_name, &cfg.players);
    assert_throw!(
        players.len() == cfg.players.len(),
        "all keygen members should attend"
    );
    let keystore = impl_keygen_mnemi(chan, i, t, players, mnem_i)
        .await
        .catch_()?;
    Ok(keystore)
}

pub async fn biz_sign(
    sesman_url: String,
    session_id: String,
    keystore: KeystoreElgamal,
    tasks: Vec<SignTask>,
) -> Resultat<Vec<Signature>> {
    assert_throw!(sesman_url.starts_with("http://") || sesman_url.starts_with("https://"));
    let https = sesman_url.starts_with("https://");

    let (chan, cfg) = SvarogChannel::use_session(&session_id, &sesman_url, https)
        .await
        .catch_()?;
    let (_, signers) = ses_arch("", &cfg.players);
    let i = keystore.i as usize;
    assert_throw!(signers.contains(&i), "signer not in the session");
    let sigs = impl_sign(chan, keystore, signers, tasks).await.catch_()?;
    Ok(sigs)
}

pub async fn biz_reshare(
    sesman_url: String,
    session_id: String,
    member_name: String,
    keystore: Option<KeystoreElgamal>,
) -> Resultat<Option<KeystoreElgamal>> {
    assert_throw!(sesman_url.starts_with("http://") || sesman_url.starts_with("https://"));
    let https = sesman_url.starts_with("https://");

    let (chan, cfg) = SvarogChannel::use_session(&session_id, &sesman_url, https)
        .await
        .catch_()?;
    let t = cfg.threshold as usize;
    let (_, providers) = ses_arch("", &cfg.players);
    if let Some(keystore) = &keystore {
        let i0 = keystore.i as usize;
        assert_throw!(providers.contains(&i0), "provider not in the session");
    }
    let (i, consumers) = ses_arch(&member_name, &cfg.players_reshared);
    assert_throw!(
        consumers.len() == cfg.players_reshared.len(),
        "all keygen members should attend"
    );
    let keystore = impl_reshare(chan, keystore, i, t, providers, consumers)
        .await
        .catch_()?;
    Ok(keystore)
}

async fn impl_keygen(
    chan: SvarogChannel,
    i: usize,
    t: usize,
    players: BTreeSet<usize>,
) -> Resultat<KeystoreElgamal> {
    let keystore = keygen(chan, players, t, i, None, None).await.catch_()?;
    Ok(keystore)
}

async fn impl_keygen_mnem(
    chan: SvarogChannel,
    i: usize,
    t: usize,
    players: BTreeSet<usize>,
    mnem: Option<Mnemonics>,
) -> Resultat<Option<KeystoreElgamal>> {
    let provider_thread = if let Some(mnem) = mnem {
        let future: _ =
            keygen_mnem_provider(chan.clone(), players.clone(), mnem.phrases, mnem.password);
        let handle: _ = tokio::spawn(future);
        Some(handle)
    } else {
        None
    };

    let ret = if i > 0 {
        let keystore = keygen_mnem_consumer(chan, players, t, i).await.catch_()?;
        Some(keystore)
    } else {
        None
    };

    if let Some(handle) = provider_thread {
        handle
            .await
            .catch("ThreadFailed", "due to panic")?
            .catch("ThreadFailed", "due to exception")?;
    }

    Ok(ret)
}

async fn impl_keygen_mnemi(
    chan: SvarogChannel,
    i: usize,
    t: usize,
    players: BTreeSet<usize>,
    mnem_i: String,
) -> Resultat<KeystoreElgamal> {
    let ui_bytes = mnemi2sk(&mnem_i).catch_()?;
    let ui = svarog_algo::k256::Scalar::from_bytes_mod_order(&ui_bytes);
    let pk = None;
    let imported = ImportedPartyKey { ui, pk };
    let keystore = keygen(chan, players, t, i, Some(imported), None)
        .await
        .catch_()?;
    Ok(keystore)
}

async fn impl_sign(
    chan: SvarogChannel,
    keystore: KeystoreElgamal,
    signers: BTreeSet<usize>,
    tasks: Vec<SignTask>,
) -> Resultat<Vec<Signature>> {
    assert_throw!(tasks.len() >= 1);
    assert_throw!(signers.len() >= 1);
    assert_throw!(signers.contains(&keystore.i));
    let res = {
        let tasks = tasks
            .into_iter()
            .map(|task| (task.message, task.bip32_path))
            .collect();
        let sigs = sign_batch(chan, signers, keystore, tasks).await.catch_()?;
        let mut res = Vec::new();
        for sig in sigs.into_iter() {
            let (r, v) = sig.eval_rv();
            let sig = Signature {
                r,
                s: *sig.s.to_bytes().as_ref(),
                v,
                pk: sig.pk.to33bytes().to_vec(),
            };
            res.push(sig);
        }
        res
    };
    Ok(res)
}

async fn impl_reshare(
    chan: SvarogChannel,
    keystore: Option<KeystoreElgamal>,
    i: usize,
    t: usize,
    providers: BTreeSet<usize>,
    consumers: BTreeSet<usize>,
) -> Resultat<Option<KeystoreElgamal>> {
    let provider_thread = if let Some(keystore) = keystore {
        let future: _ =
            reshare_provider(chan.clone(), keystore, providers.clone(), consumers.clone());
        let handle: _ = tokio::spawn(future);
        Some(handle)
    } else {
        None
    };

    let ret = if i > 0 {
        let keystore = reshare_consumer(chan, t, i, providers, consumers)
            .await
            .catch_()?;
        Some(keystore)
    } else {
        None
    };

    if let Some(handle) = provider_thread {
        handle
            .await
            .catch("ThreadFailed", "due to panic")?
            .catch("ThreadFailed", "due to exception")?;
    }

    Ok(ret)
}
