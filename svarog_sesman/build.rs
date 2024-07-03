use erreur::*;
use vergen::EmitBuilder;

pub fn main() -> Resultat<()> {
    EmitBuilder::builder().all_git().emit().catch_()?;
    Ok(())
}
