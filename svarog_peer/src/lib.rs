use std::collections::{BTreeMap, BTreeSet, HashMap};

use erreur::*;
use svarog_grpc::SessionConfig;
use svarog_sesman::SvarogChannel;

pub mod btc;
pub use btc as eth;
pub mod solana;
pub mod structs;

pub async fn new_session(cfg: SessionConfig) -> Resultat<String> {
    assert_throw!(cfg.sesman_url.starts_with("http://") || cfg.sesman_url.starts_with("https://"));
    let https = cfg.sesman_url.starts_with("https://");

    let chan = SvarogChannel::new_session(&cfg, &cfg.sesman_url, https)
        .await
        .catch_()?;
    let sid = chan.sid().to_owned();

    Ok(sid)
}

fn ses_arch(name: &str, names: &HashMap<String, bool>) -> (usize, BTreeSet<usize>) {
    let names: BTreeMap<String, bool> = names.iter().map(|(k, v)| (k.clone(), *v)).collect();
    let mut i = 0;
    let mut players = BTreeSet::new();
    for (j, (_name, &att)) in names.iter().enumerate() {
        if att {
            let j = j + 1;
            players.insert(j);
            if name == _name {
                i = j;
            }
        }
    }
    (i, players)
}

pub fn version() -> String {
    format!(
        "svarog_service git commit id: {}, branch: {}",
        env!("VERGEN_GIT_SHA"),
        env!("VERGEN_GIT_BRANCH")
    )
}
