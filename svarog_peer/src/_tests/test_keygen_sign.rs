#![allow(nonstandard_style)]
use std::collections::BTreeMap;

use erreur::*;
use mock_data::mock_sign_tasks;
use svarog_peer::{btc, new_session, solana};

// 改成通配符引用之后, 会难以检查到底用了哪些符号. 通配符看着优雅, 但是不利于代码审查.
use crate::mock_data::{mock_keygen_config, mock_sign_config, players1, th1};

mod mock_data;
const sesman_url: &str = "http://127.0.0.1:2000";

/// 集成测试普通的keygen, sign
#[tokio::main]
async fn main() -> Resultat<()> {
    println!("{}", svarog_peer::version());
    println!("{}", svarog_algo::version());
    test_btc().await.catch_()?;
    test_solana().await.catch_()?;
    Ok(())
}

async fn test_btc() -> Resultat<()> {
    let keystores = {
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

    let signatures = {
        let cfg = mock_sign_config(th1, &players1, sesman_url);
        let sid = new_session(cfg.clone()).await.catch_()?;
        let mut threads = BTreeMap::new();
        for (player, &att) in cfg.players.iter() {
            if false == att {
                continue;
            }
            let keystore = keystores.get(player).ifnone_()?;
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

    // let tasks = mock_sign_tasks();
    // for i in 0..tasks.len() {
    //     let m = &tasks[i].message;
    //     let m = hex::encode(m);

    //     let sig = &sig0[i];
    //     let mut rsv_bytes = [0u8; 65];
    //     rsv_bytes[0..32].copy_from_slice(&sig.r);
    //     rsv_bytes[32..64].copy_from_slice(&sig.s);
    //     rsv_bytes[64] = sig.v;
    //     let rsv = hex::encode(rsv_bytes);

    //     let pk = hex::encode(&sig.pk);
    //     println!("--hash {} --rsv {} --pk {}", m, rsv, pk);
    // }

    Ok(())
}

async fn test_solana() -> Resultat<()> {
    let keystores = {
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

    let signatures = {
        let cfg = mock_sign_config(th1, &players1, sesman_url);
        let sid = new_session(cfg.clone()).await.catch_()?;
        let mut threads = BTreeMap::new();
        for (player, &att) in cfg.players.iter() {
            if false == att {
                continue;
            }
            let keystore = keystores.get(player).ifnone_()?;
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
