#![allow(nonstandard_style)]
use std::collections::BTreeMap;

use erreur::*;
use mock_data::mock_sign_tasks;
use svarog_peer::{btc, new_session, solana};

use crate::mock_data::{
    mock_keygen_config, mock_reshare_config, mock_sign_config, players1, players2, th1, th2,
};

mod mock_data;
const sesman_url: &str = "http://127.0.0.1:2000";

/// 集成测试普通的keygen, sign
#[tokio::main]
async fn main() -> Resultat<()> {
    test_btc().await.catch_()?;
    test_solana().await.catch_()?;
    Ok(())
}

async fn test_btc() -> Resultat<()> {
    let keystores_old = {
        let cfg = mock_keygen_config(th1, &players1, sesman_url);
        let sid = new_session(cfg.clone()).await.catch_()?;
        let mut threads = BTreeMap::new();
        for (player, _) in cfg.players.iter() {
            let future = btc::biz_keygen(sesman_url.to_owned(), sid.clone(), player.clone());
            let thread = tokio::spawn(future);
            threads.insert(player.clone(), thread);
        }
        let mut keystores = BTreeMap::new();
        for (player, thread) in threads.iter_mut() {
            let resp = thread.await.catch("Panic", "")?.catch("Exception", "")?;
            keystores.insert(player.clone(), resp);
        }
        keystores
    };

    let keystores = {
        let (cfg, exclusive_consumers) =
            mock_reshare_config(th1, &players1, th2, &players2, sesman_url);
        let sid = new_session(cfg.clone()).await.catch_()?;

        // spawn thread for reshare providers
        let mut threads = BTreeMap::new();
        for (player, &att) in cfg.players.iter() {
            if false == att {
                continue;
            }
            let keystore = keystores_old.get(player).ifnone_()?;
            let future = btc::biz_reshare(
                sesman_url.to_owned(),
                sid.clone(),
                player.clone(),
                Some(keystore.clone()),
            );
            let thread = tokio::spawn(future);
            threads.insert(player.clone(), thread);
        }

        // spawn threads for reshare consumers not in providers
        for player in exclusive_consumers.iter() {
            let future = btc::biz_reshare(sesman_url.to_owned(), sid.clone(), player.clone(), None);
            let thread = tokio::spawn(future);
            threads.insert(player.clone(), thread);
        }

        let mut keystores = BTreeMap::new();
        for (player, thread) in threads.iter_mut() {
            let resp = thread.await.catch("Panic", "")?.catch("Exception", "")?;
            keystores.insert(player.clone(), resp);
        }
        keystores
    };

    let signatures = {
        let cfg = mock_sign_config(th2, &players2, sesman_url);
        let sid = new_session(cfg.clone()).await.catch_()?;
        let mut threads = BTreeMap::new();
        for (player, &att) in cfg.players.iter() {
            if false == att {
                continue;
            }
            let keystore = keystores.get(player).ifnone_()?.as_ref().ifnone_()?;
            let future = btc::biz_sign(
                sesman_url.to_owned(),
                sid.clone(),
                keystore.clone(),
                mock_sign_tasks(),
            );
            let thread = tokio::spawn(future);
            threads.insert(player, thread);
        }
        let mut signatures = BTreeMap::new();
        for (&player, thread) in threads.iter_mut() {
            let resp = thread.await.catch("Panic", "")?.catch("Exception", "")?;
            signatures.insert(player.clone(), resp);
        }
        signatures
    };

    let mut sig_it = signatures.values();
    let sig0 = sig_it.next().ifnone_()?;
    for sig in sig_it {
        assert_throw!(sig == sig0);
    }
    Ok(())
}

async fn test_solana() -> Resultat<()> {
    let keystores_old = {
        let cfg = mock_keygen_config(th1, &players1, sesman_url);
        let sid = new_session(cfg.clone()).await.catch_()?;
        let mut threads = BTreeMap::new();
        for (player, _) in cfg.players.iter() {
            let future = solana::biz_keygen(sesman_url.to_owned(), sid.clone(), player.clone());
            let thread = tokio::spawn(future);
            threads.insert(player.clone(), thread);
        }
        let mut keystores = BTreeMap::new();
        for (player, thread) in threads.iter_mut() {
            let resp = thread.await.catch("Panic", "")?.catch("Exception", "")?;
            keystores.insert(player.clone(), resp);
        }
        keystores
    };

    let keystores = {
        let (cfg, exclusive_consumers) =
            mock_reshare_config(th1, &players1, th2, &players2, sesman_url);
        let sid = new_session(cfg.clone()).await.catch_()?;

        // spawn thread for reshare providers
        let mut threads = BTreeMap::new();
        for (player, &att) in cfg.players.iter() {
            if false == att {
                continue;
            }
            let keystore = keystores_old.get(player).ifnone_()?;
            let future = solana::biz_reshare(
                sesman_url.to_owned(),
                sid.clone(),
                player.clone(),
                Some(keystore.clone()),
            );
            let thread = tokio::spawn(future);
            threads.insert(player.clone(), thread);
        }

        // spawn threads for reshare consumers not in providers
        for player in exclusive_consumers.iter() {
            let future =
                solana::biz_reshare(sesman_url.to_owned(), sid.clone(), player.clone(), None);
            let thread = tokio::spawn(future);
            threads.insert(player.clone(), thread);
        }

        let mut keystores = BTreeMap::new();
        for (player, thread) in threads.iter_mut() {
            let resp = thread.await.catch("Panic", "")?.catch("Exception", "")?;
            keystores.insert(player.clone(), resp);
        }
        keystores
    };

    let signatures = {
        let cfg = mock_sign_config(th2, &players2, sesman_url);
        let sid = new_session(cfg.clone()).await.catch_()?;
        let mut threads = BTreeMap::new();
        for (player, &att) in cfg.players.iter() {
            if false == att {
                continue;
            }
            let keystore = keystores.get(player).ifnone_()?.as_ref().ifnone_()?;
            let future = solana::biz_sign(
                sesman_url.to_owned(),
                sid.clone(),
                keystore.clone(),
                mock_sign_tasks(),
            );
            let thread = tokio::spawn(future);
            threads.insert(player, thread);
        }
        let mut signatures = BTreeMap::new();
        for (&player, thread) in threads.iter_mut() {
            let resp = thread.await.catch("Panic", "")?.catch("Exception", "")?;
            signatures.insert(player.clone(), resp);
        }
        signatures
    };

    let mut sig_it = signatures.values();
    let sig0 = sig_it.next().ifnone_()?;
    for sig in sig_it {
        assert_throw!(sig == sig0);
    }
    Ok(())
}
